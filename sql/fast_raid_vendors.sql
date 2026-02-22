-- Fast raid-prep vendors for AzerothCore (Wrath)
-- Purpose: spawn one vendor per class + misc vendors and auto-populate items
-- for quick weekend gearing.

SET @MAP := 571; -- Dalaran
SET @ZONE := 4395;
SET @AREA := 4395;
SET @X := 5815.0;
SET @Y := 650.0;
SET @Z := 647.0;
SET @O := 2.50;

SET @BASE := 910000;
SET @WARRIOR := @BASE + 1;
SET @PALADIN := @BASE + 2;
SET @HUNTER := @BASE + 3;
SET @ROGUE := @BASE + 4;
SET @PRIEST := @BASE + 5;
SET @DK := @BASE + 6;
SET @SHAMAN := @BASE + 7;
SET @MAGE := @BASE + 8;
SET @WARLOCK := @BASE + 9;
SET @DRUID := @BASE + 10;
SET @UNIVERSAL := @BASE + 50;
SET @CONSUMABLES := @BASE + 51;
SET @GEMS_ENCHANTS := @BASE + 52;
SET @MOUNTS_MISC := @BASE + 53;

-- Cleanup existing templates/spawns/vendor rows from previous runs.
DELETE FROM npc_vendor WHERE `entry` BETWEEN @BASE AND (@BASE + 100);
DELETE FROM creature WHERE `id1` BETWEEN @BASE AND (@BASE + 100);
DELETE FROM creature_template WHERE `entry` BETWEEN @BASE AND (@BASE + 100);

-- Create templates (vendor flag 128).
INSERT INTO creature_template
(`entry`,`modelid1`,`name`,`subname`,`minlevel`,`maxlevel`,`faction`,`npcflag`,`speed_walk`,`speed_run`,`scale`,`rank`,`unit_class`,`unit_flags`,`type`,`type_flags`,`RegenHealth`)
VALUES
(@WARRIOR, 19723, 'Warrior BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@PALADIN, 19724, 'Paladin BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@HUNTER, 19725, 'Hunter BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@ROGUE, 19726, 'Rogue BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@PRIEST, 19727, 'Priest BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@DK, 19728, 'Death Knight BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@SHAMAN, 19729, 'Shaman BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@MAGE, 19730, 'Mage BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@WARLOCK, 19731, 'Warlock BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@DRUID, 19732, 'Druid BiS Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@UNIVERSAL, 19733, 'Universal Epic Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@CONSUMABLES, 19734, 'Consumables Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@GEMS_ENCHANTS, 19735, 'Gems & Enchants Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1),
(@MOUNTS_MISC, 19736, 'Mounts & Misc Vendor', 'Weekend Raid Prep', 80, 80, 35, 128, 1, 1.14286, 1, 0, 1, 0, 7, 0, 1);

-- Spawn in Dalaran, compact layout.
INSERT INTO creature
(`id1`,`map`,`zoneId`,`areaId`,`position_x`,`position_y`,`position_z`,`orientation`,`spawntimesecs`,`spawndist`,`MovementType`)
VALUES
(@WARRIOR, @MAP, @ZONE, @AREA, @X + 0.0,  @Y + 0.0, @Z, @O, 120, 0, 0),
(@PALADIN, @MAP, @ZONE, @AREA, @X + 1.7,  @Y + 0.0, @Z, @O, 120, 0, 0),
(@HUNTER,  @MAP, @ZONE, @AREA, @X + 3.4,  @Y + 0.0, @Z, @O, 120, 0, 0),
(@ROGUE,   @MAP, @ZONE, @AREA, @X + 5.1,  @Y + 0.0, @Z, @O, 120, 0, 0),
(@PRIEST,  @MAP, @ZONE, @AREA, @X + 6.8,  @Y + 0.0, @Z, @O, 120, 0, 0),
(@DK,      @MAP, @ZONE, @AREA, @X + 0.0,  @Y + 2.0, @Z, @O, 120, 0, 0),
(@SHAMAN,  @MAP, @ZONE, @AREA, @X + 1.7,  @Y + 2.0, @Z, @O, 120, 0, 0),
(@MAGE,    @MAP, @ZONE, @AREA, @X + 3.4,  @Y + 2.0, @Z, @O, 120, 0, 0),
(@WARLOCK, @MAP, @ZONE, @AREA, @X + 5.1,  @Y + 2.0, @Z, @O, 120, 0, 0),
(@DRUID,   @MAP, @ZONE, @AREA, @X + 6.8,  @Y + 2.0, @Z, @O, 120, 0, 0),
(@UNIVERSAL,   @MAP, @ZONE, @AREA, @X + 0.0, @Y + 4.3, @Z, @O, 120, 0, 0),
(@CONSUMABLES, @MAP, @ZONE, @AREA, @X + 2.3, @Y + 4.3, @Z, @O, 120, 0, 0),
(@GEMS_ENCHANTS, @MAP, @ZONE, @AREA, @X + 4.6, @Y + 4.3, @Z, @O, 120, 0, 0),
(@MOUNTS_MISC, @MAP, @ZONE, @AREA, @X + 6.9, @Y + 4.3, @Z, @O, 120, 0, 0);

-- Class bitmasks (Wrath):
-- 1 Warrior, 2 Paladin, 4 Hunter, 8 Rogue, 16 Priest, 32 Death Knight,
-- 64 Shaman, 128 Mage, 256 Warlock, 1024 Druid.

-- One class vendor each: top epic class-usable gear/weapons (ilvl >= 245).
SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @WARRIOR, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 1) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @PALADIN, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 2) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @HUNTER, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 4) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @ROGUE, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 8) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @PRIEST, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 16) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @DK, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 32) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @SHAMAN, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 64) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @MAGE, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 128) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @WARLOCK, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 256) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @DRUID, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 245 AND it.class IN (2,4)
  AND (it.AllowableClass = -1 OR (it.AllowableClass & 1024) <> 0)
ORDER BY it.ItemLevel DESC, it.Quality DESC, it.entry DESC
LIMIT 250;

-- Universal epic vendor (all classes, high ilvl gear/weapons).
SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @UNIVERSAL, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.Quality >= 4 AND it.ItemLevel >= 251 AND it.class IN (2,4)
ORDER BY it.ItemLevel DESC, it.entry DESC
LIMIT 250;

-- Consumables vendor.
SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @CONSUMABLES, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.class = 0 AND it.RequiredLevel <= 80
  AND (
    it.name LIKE '%Flask%' OR
    it.name LIKE '%Potion%' OR
    it.name LIKE '%Elixir%' OR
    it.name LIKE '%Bandage%' OR
    it.name LIKE '%Food%' OR
    it.name LIKE '%Feast%'
  )
ORDER BY it.ItemLevel DESC, it.entry DESC
LIMIT 250;

-- Gems + enchanting/trade utility.
SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @GEMS_ENCHANTS, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE (it.class = 3 OR it.class = 7)
  AND it.Quality >= 2
ORDER BY it.ItemLevel DESC, it.entry DESC
LIMIT 250;

-- Mounts and convenience misc.
SET @slot := -1;
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`)
SELECT @MOUNTS_MISC, (@slot := @slot + 1), it.entry, 0, 0, 0, 1, NULL, 0, 0
FROM item_template it
WHERE it.class IN (15, 12)
ORDER BY it.ItemLevel DESC, it.entry DESC
LIMIT 250;
