-- FULLTEXT index on item_template.name to accelerate the API's item search.
-- Idempotent: safe to run multiple times. Apply to the acore_world database,
-- e.g. `mysql -uroot -pPASSWORD acore_world < sql/fulltext_item_template_name.sql`
-- (or via scripts/seed-fast-raid-vendors.sh).

SET @schema_name = DATABASE();
SET @index_name = 'idx_item_template_name_fulltext';

SET @sql = IF(
    EXISTS(
        SELECT 1 FROM information_schema.STATISTICS
        WHERE TABLE_SCHEMA = @schema_name
          AND TABLE_NAME = 'item_template'
          AND INDEX_NAME = @index_name
    ),
    'SELECT 1',
    'CREATE FULLTEXT INDEX idx_item_template_name_fulltext ON item_template (name)'
);

PREPARE stmt FROM @sql;
EXECUTE stmt;
DEALLOCATE PREPARE stmt;
