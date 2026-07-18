-- One audit trigger function, attached AFTER INSERT/UPDATE/DELETE to
-- quotes, signals, opportunities. actor comes from the same transaction-local
-- GUC the RLS policies read — NULL when the seed (admin connection) writes.

CREATE FUNCTION audit_row_change() RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_actor uuid := NULLIF(current_setting('app.user_id', true), '')::uuid;
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO audit_log (actor, action, entity, entity_id, before, after, at)
        VALUES (v_actor, TG_OP, TG_TABLE_NAME, NEW.id, NULL, to_jsonb(NEW), now());
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO audit_log (actor, action, entity, entity_id, before, after, at)
        VALUES (v_actor, TG_OP, TG_TABLE_NAME, NEW.id, to_jsonb(OLD), to_jsonb(NEW), now());
        RETURN NEW;
    ELSE
        INSERT INTO audit_log (actor, action, entity, entity_id, before, after, at)
        VALUES (v_actor, TG_OP, TG_TABLE_NAME, OLD.id, to_jsonb(OLD), NULL, now());
        RETURN OLD;
    END IF;
END;
$$;

CREATE TRIGGER quotes_audit
AFTER INSERT OR UPDATE OR DELETE ON quotes
FOR EACH ROW EXECUTE FUNCTION audit_row_change();

CREATE TRIGGER signals_audit
AFTER INSERT OR UPDATE OR DELETE ON signals
FOR EACH ROW EXECUTE FUNCTION audit_row_change();

CREATE TRIGGER opportunities_audit
AFTER INSERT OR UPDATE OR DELETE ON opportunities
FOR EACH ROW EXECUTE FUNCTION audit_row_change();
