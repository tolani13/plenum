//! Ask PLENUM's SQL validator (R9) — a PURE function over the sqlparser AST.
//! No database, no HTTP, no key: adversarial unit tests live right here.
//!
//! The contract, enforced in order:
//!   1. exactly ONE statement, and it is a Query — INSERT/UPDATE/DELETE/DDL/
//!      COPY/SET/EXPLAIN are different Statement variants and die here;
//!      "SELECT … INTO" and locking clauses (FOR UPDATE/SHARE) are Query
//!      forms with write/lock semantics and die here too;
//!   2. every relation referenced ANYWHERE in the tree (FROM, joins,
//!      subqueries in any expression position, set operations, CTE bodies)
//!      is either a whitelisted view or a CTE defined by the query itself;
//!   3. table functions in FROM (UNNEST, json_table, srf calls) are refused —
//!      relations are plain tables/views or derived subqueries only;
//!   4. a short function denylist (set_config, backend signals, file reads,
//!      the *_to_xml family that executes SQL strings) — belt-and-braces
//!      UNDER the read-only transaction and the role's grants, not instead
//!      of them.
//!
//! On success the CANONICAL serialization of the parsed statement is
//! returned (comments and stray semicolons do not survive), and THAT string
//! is what gets wrapped and executed — the validated bytes, not the model's.

use std::collections::HashSet;
use std::ops::ControlFlow;

use sqlparser::ast::{Expr, ObjectName, Query, SetExpr, Statement, TableFactor, Visit, Visitor};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

/// The whitelisted semantic layer (spec §6.5): the facts view, the four
/// scoped rollup views, and the defection view. Raw tables and mv_* never
/// appear here — the mv_* have no plenum_app grant anyway (defense in depth).
pub const WHITELIST: [&str; 6] = [
    "v_order_facts",
    "v_territory_period",
    "v_rep_period",
    "v_product_period",
    "v_customer_period",
    "v_defection_risk",
];

/// Functions refused outright. READ ONLY blocks writes and the role's grants
/// block the catalogs, but these either mutate session state (set_config —
/// could re-point the RLS GUC mid-statement), signal backends, touch files,
/// or execute SQL from strings (the *_to_xml family) — none has any place in
/// an analytics question.
const FN_DENYLIST: [&str; 8] = [
    "set_config",
    "pg_cancel_backend",
    "pg_terminate_backend",
    "pg_reload_conf",
    "pg_rotate_logfile",
    "pg_read_file",
    "pg_read_binary_file",
    "pg_ls_dir",
];

struct AskVisitor {
    /// CTE names defined by the query — legal relation references.
    ctes: HashSet<String>,
    violation: Option<String>,
}

fn last_ident_lower(name: &ObjectName) -> String {
    name.0
        .last()
        .map(|p| p.to_string().trim_matches('"').to_ascii_lowercase())
        .unwrap_or_default()
}

impl AskVisitor {
    fn fail(&mut self, msg: String) -> ControlFlow<()> {
        if self.violation.is_none() {
            self.violation = Some(msg);
        }
        ControlFlow::Break(())
    }
}

impl Visitor for AskVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        // Register CTE names before their bodies/references are visited
        // (pre-order). A CTE may shadow a whitelisted name — that is safe:
        // Postgres resolves the reference to the CTE, whose own body was
        // itself walked against this same rule.
        if let Some(with) = &query.with {
            for cte in &with.cte_tables {
                self.ctes.insert(cte.alias.name.value.to_ascii_lowercase());
            }
        }
        // SELECT … INTO is a write dressed as a query.
        if let SetExpr::Select(select) = query.body.as_ref() {
            if select.into.is_some() {
                return self.fail("SELECT INTO is not allowed".into());
            }
        }
        // VALUES / bare tables / nested inserts as a query body: refuse
        // anything that is not a plain select tree or a set operation.
        match query.body.as_ref() {
            SetExpr::Select(_) | SetExpr::Query(_) | SetExpr::SetOperation { .. } => {}
            _ => return self.fail("only SELECT queries are allowed".into()),
        }
        // FOR UPDATE / FOR SHARE lock rows — meaningless in analytics, and a
        // write-adjacent smell.
        if !query.locks.is_empty() {
            return self.fail("locking clauses are not allowed".into());
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, table_factor: &TableFactor) -> ControlFlow<Self::Break> {
        match table_factor {
            TableFactor::Table {
                args: Some(_),
                name,
                ..
            } => self.fail(format!(
                "table functions are not allowed: {}",
                last_ident_lower(name)
            )),
            TableFactor::Table { .. }
            | TableFactor::Derived { .. }
            | TableFactor::NestedJoin { .. } => ControlFlow::Continue(()),
            _ => self.fail("only plain tables and subqueries may appear in FROM".into()),
        }
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        let name = last_ident_lower(relation);
        if self.ctes.contains(&name) || WHITELIST.contains(&name.as_str()) {
            ControlFlow::Continue(())
        } else {
            self.fail(format!(
                "relation \"{name}\" is not in the whitelisted semantic layer"
            ))
        }
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(f) = expr {
            let name = last_ident_lower(&f.name);
            if FN_DENYLIST.contains(&name.as_str()) || name.contains("to_xml") {
                return self.fail(format!("function \"{name}\" is not allowed"));
            }
        }
        ControlFlow::Continue(())
    }
}

/// Validate a model-generated SQL string. Ok(canonical_sql) or Err(a message
/// safe to show the caller in a typed 422).
pub fn validate_ask_sql(sql: &str) -> Result<String, String> {
    let statements = Parser::parse_sql(&PostgreSqlDialect {}, sql)
        .map_err(|e| format!("could not parse the generated SQL: {e}"))?;

    if statements.len() != 1 {
        return Err(format!(
            "exactly one statement is allowed (got {})",
            statements.len()
        ));
    }
    let statement = &statements[0];

    let Statement::Query(_) = statement else {
        return Err("only a single SELECT query is allowed".into());
    };

    let mut visitor = AskVisitor {
        ctes: HashSet::new(),
        violation: None,
    };
    let _ = statement.visit(&mut visitor);
    if let Some(v) = visitor.violation {
        return Err(v);
    }

    Ok(statement.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(sql: &str) {
        assert!(
            validate_ask_sql(sql).is_ok(),
            "expected ACCEPT: {sql}\n  got: {:?}",
            validate_ask_sql(sql)
        );
    }

    fn rejected(sql: &str, needle: &str) {
        match validate_ask_sql(sql) {
            Ok(s) => panic!("expected REJECT: {sql}\n  but validated as: {s}"),
            Err(e) => assert!(
                e.to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase()),
                "expected message containing {needle:?}, got {e:?}"
            ),
        }
    }

    // ── writes and DDL die at the statement gate ────────────────────────────

    #[test]
    fn rejects_writes_and_ddl() {
        rejected("INSERT INTO signals (id) VALUES (1)", "single SELECT");
        rejected("UPDATE quotes SET status = 'approved'", "single SELECT");
        rejected("DELETE FROM audit_log", "single SELECT");
        rejected("DROP TABLE orders", "single SELECT");
        rejected("TRUNCATE orders", "single SELECT");
        rejected("CREATE TABLE x (id int)", "single SELECT");
        rejected("COPY orders TO '/tmp/x'", "single SELECT");
        rejected("SET role postgres", "single SELECT");
        rejected("EXPLAIN SELECT * FROM v_order_facts", "single SELECT");
        rejected("GRANT SELECT ON orders TO PUBLIC", "single SELECT");
    }

    // ── the whitelist holds, everywhere a relation can hide ─────────────────

    #[test]
    fn rejects_off_whitelist_relations() {
        rejected("SELECT * FROM users", "not in the whitelisted");
        rejected("SELECT * FROM orders", "not in the whitelisted");
        rejected("SELECT * FROM mv_rep_period", "not in the whitelisted");
        rejected("SELECT * FROM audit_log", "not in the whitelisted");
        rejected("SELECT * FROM public.users", "not in the whitelisted");
        rejected(
            "SELECT * FROM v_order_facts f JOIN users u ON u.id = f.rep_id",
            "not in the whitelisted",
        );
        rejected(
            "SELECT (SELECT password_hash FROM users LIMIT 1)",
            "not in the whitelisted",
        );
        rejected(
            "SELECT * FROM v_order_facts WHERE rep_id IN (SELECT id FROM users)",
            "not in the whitelisted",
        );
        rejected(
            "WITH x AS (SELECT * FROM users) SELECT * FROM x",
            "not in the whitelisted",
        );
        rejected(
            "SELECT * FROM v_order_facts UNION ALL SELECT * FROM pg_shadow",
            "not in the whitelisted",
        );
    }

    // ── multi-statement / smuggling ─────────────────────────────────────────

    #[test]
    fn rejects_multi_statement_and_smuggling() {
        rejected(
            "SELECT * FROM v_order_facts; DROP TABLE orders",
            "exactly one statement",
        );
        rejected(
            "SELECT 1; SELECT * FROM v_order_facts",
            "exactly one statement",
        );
        // A trailing semicolon is ONE statement — allowed, and the canonical
        // form drops it (nothing left to smuggle behind).
        let canonical = validate_ask_sql("SELECT territory_code FROM v_territory_period;")
            .expect("trailing semicolon is fine");
        assert!(!canonical.contains(';'), "canonical form has no semicolon");
    }

    // ── write-adjacent query forms ──────────────────────────────────────────

    #[test]
    fn rejects_select_into_locks_and_table_functions() {
        rejected("SELECT * INTO new_table FROM v_order_facts", "SELECT INTO");
        rejected("SELECT * FROM v_order_facts FOR UPDATE", "locking");
        rejected("SELECT * FROM generate_series(1, 10)", "table function");
    }

    // ── the function denylist ───────────────────────────────────────────────

    #[test]
    fn rejects_denylisted_functions() {
        rejected("SELECT set_config('app.user_id', 'x', true)", "not allowed");
        rejected(
            "SELECT query_to_xml('SELECT * FROM users', true, true, '')",
            "not allowed",
        );
        rejected("SELECT pg_read_file('/etc/passwd')", "not allowed");
    }

    // ── the legitimate shapes all pass ──────────────────────────────────────

    #[test]
    fn accepts_whitelisted_queries() {
        ok("SELECT account_name, SUM(net_cents) AS net FROM v_order_facts GROUP BY account_name ORDER BY net DESC LIMIT 10");
        ok("SELECT * FROM v_defection_risk ORDER BY score DESC");
        ok("SELECT territory_code, SUM(net_cents) FROM v_territory_period WHERE quarter_start >= '2025-01-01' AND quarter_start < '2026-01-01' GROUP BY territory_code");
        // CTE over the whitelist, referenced downstream.
        ok("WITH y AS (SELECT account_name, net_cents FROM v_customer_period) SELECT account_name, SUM(net_cents) FROM y GROUP BY account_name");
        // Derived subquery + join between whitelisted views.
        ok("SELECT f.rep_name, t.gross_cents FROM v_rep_period f JOIN (SELECT territory_id, gross_cents FROM v_territory_period) t ON t.territory_id = f.territory_id");
        // A CTE may shadow a whitelisted name; its body is still checked.
        ok("WITH v_order_facts AS (SELECT 1 AS x) SELECT x FROM v_order_facts");
    }

    // ── canonicalization strips comments ────────────────────────────────────

    #[test]
    fn canonical_form_is_comment_free() {
        let canonical =
            validate_ask_sql("SELECT year /* sneaky */ FROM v_order_facts -- trailing\nLIMIT 5")
                .expect("comments are legal, just not preserved");
        assert!(!canonical.contains("sneaky"));
        assert!(!canonical.contains("--"));
    }
}
