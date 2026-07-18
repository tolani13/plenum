-- v_user_scope: (user_id, territory_id) pairs a user is allowed to see.
--
-- A rep's subtree is just themselves — their own territory_assignments.
-- A regional manager's subtree pulls their reports' assignments.
-- vp/admin get the cross join (all territories).
-- The recursion terminates because manager chains form a tree (reps have no
-- reports; the chain is rep -> regional_manager -> vp).

CREATE VIEW v_user_scope AS
WITH RECURSIVE subordinates AS (
  SELECT id, id AS root FROM users
  UNION ALL
  SELECT u.id, s.root FROM users u JOIN subordinates s ON u.manager_id = s.id
)
SELECT u.id AS user_id, t.id AS territory_id
FROM users u
CROSS JOIN territories t
WHERE u.role IN ('vp','admin')
UNION
SELECT s.root AS user_id, ta.territory_id
FROM subordinates s
JOIN territory_assignments ta ON ta.user_id = s.id;
