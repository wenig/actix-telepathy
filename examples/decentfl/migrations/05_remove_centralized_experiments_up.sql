--DELETE FROM evaluations WHERE experiment_id IN (SELECT id FROM experiments WHERE centralized IS TRUE);
DELETE FROM experiments WHERE centralized IS TRUE;