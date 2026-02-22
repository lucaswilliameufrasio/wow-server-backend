-- ICC25H-style fixed preset vendors (manual, deterministic item IDs)
-- This complements fast_raid_vendors.sql with curated starter packs.

SET @MAP := 571;
SET @ZONE := 4395;
SET @AREA := 4395;
SET @X := 5830.0;
SET @Y := 650.0;
SET @Z := 647.0;
SET @O := 2.50;

SET @BASE := 920000;
SET @PHYS := @BASE + 1;
SET @CASTER := @BASE + 2;
SET @HEALER := @BASE + 3;
SET @TANK := @BASE + 4;
SET @RANGED := @BASE + 5;

DELETE FROM npc_vendor WHERE `entry` BETWEEN @BASE AND (@BASE + 20);
DELETE FROM creature WHERE `id1` BETWEEN @BASE AND (@BASE + 20);
DELETE FROM creature_template WHERE `entry` BETWEEN @BASE AND (@BASE + 20);

INSERT INTO creature_template
(`entry`,`modelid1`,`name`,`subname`,`minlevel`,`maxlevel`,`faction`,`npcflag`,`speed_walk`,`speed_run`,`scale`,`unit_class`,`type`)
VALUES
(@PHYS, 24972, 'Preset BiS: Melee DPS', 'ICC25H Starter Pack', 80, 80, 35, 128, 1, 1.14286, 1, 1, 7),
(@CASTER, 24973, 'Preset BiS: Caster DPS', 'ICC25H Starter Pack', 80, 80, 35, 128, 1, 1.14286, 1, 1, 7),
(@HEALER, 24974, 'Preset BiS: Healer', 'ICC25H Starter Pack', 80, 80, 35, 128, 1, 1.14286, 1, 1, 7),
(@TANK, 24975, 'Preset BiS: Tank', 'ICC25H Starter Pack', 80, 80, 35, 128, 1, 1.14286, 1, 1, 7),
(@RANGED, 24976, 'Preset BiS: Ranged Physical', 'ICC25H Starter Pack', 80, 80, 35, 128, 1, 1.14286, 1, 1, 7);

INSERT INTO creature
(`id1`,`map`,`zoneId`,`areaId`,`position_x`,`position_y`,`position_z`,`orientation`,`spawntimesecs`,`spawndist`,`MovementType`)
VALUES
(@PHYS,   @MAP, @ZONE, @AREA, @X + 0.0, @Y + 0.0, @Z, @O, 120, 0, 0),
(@CASTER, @MAP, @ZONE, @AREA, @X + 1.7, @Y + 0.0, @Z, @O, 120, 0, 0),
(@HEALER, @MAP, @ZONE, @AREA, @X + 3.4, @Y + 0.0, @Z, @O, 120, 0, 0),
(@TANK,   @MAP, @ZONE, @AREA, @X + 5.1, @Y + 0.0, @Z, @O, 120, 0, 0),
(@RANGED, @MAP, @ZONE, @AREA, @X + 6.8, @Y + 0.0, @Z, @O, 120, 0, 0);

-- Melee DPS presets (Warrior/Ret/DK/Rogue/Feral)
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`) VALUES
(@PHYS, 0, 49623, 0, 0, 0, 1, NULL, 0, 0), -- Shadowmourne
(@PHYS, 1, 50730, 0, 0, 0, 1, NULL, 0, 0), -- Glorenzelg
(@PHYS, 2, 50362, 0, 0, 0, 1, NULL, 0, 0), -- Deathbringer's Will
(@PHYS, 3, 50351, 0, 0, 0, 1, NULL, 0, 0), -- Tiny Abomination in a Jar
(@PHYS, 4, 50355, 0, 0, 0, 1, NULL, 0, 0), -- Herkuml War Token
(@PHYS, 5, 50402, 0, 0, 0, 1, NULL, 0, 0), -- Ashen Band of Endless Vengeance
(@PHYS, 6, 50713, 0, 0, 0, 1, NULL, 0, 0), -- Crimson Star
(@PHYS, 7, 50762, 0, 0, 0, 1, NULL, 0, 0); -- Cataclysmic Chestguard

-- Caster DPS presets (Mage/Warlock/Boomkin/Ele/Shadow)
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`) VALUES
(@CASTER, 0, 50365, 0, 0, 0, 1, NULL, 0, 0), -- Phylactery of the Nameless Lich
(@CASTER, 1, 50353, 0, 0, 0, 1, NULL, 0, 0), -- Dislodged Foreign Object
(@CASTER, 2, 50731, 0, 0, 0, 1, NULL, 0, 0), -- Archus, Greatstaff of Antonidas
(@CASTER, 3, 50781, 0, 0, 0, 1, NULL, 0, 0), -- Coldwraith Links
(@CASTER, 4, 50714, 0, 0, 0, 1, NULL, 0, 0), -- Lich Wrappings
(@CASTER, 5, 50469, 0, 0, 0, 1, NULL, 0, 0), -- Volde's Cloak of the Night Sky
(@CASTER, 6, 50458, 0, 0, 0, 1, NULL, 0, 0), -- Bizuri's Totem of Shattered Ice
(@CASTER, 7, 50409, 0, 0, 0, 1, NULL, 0, 0); -- Ring of Rotting Sinew

-- Healer presets (Holy/Disc/Resto)
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`) VALUES
(@HEALER, 0, 50366, 0, 0, 0, 1, NULL, 0, 0), -- Althor's Abacus
(@HEALER, 1, 50360, 0, 0, 0, 1, NULL, 0, 0), -- Phylactery (heroic alt trinket placeholder)
(@HEALER, 2, 50725, 0, 0, 0, 1, NULL, 0, 0), -- Dying Light
(@HEALER, 3, 50724, 0, 0, 0, 1, NULL, 0, 0), -- Bloodsurge, Kel'Thuzad's Blade of Agony
(@HEALER, 4, 50616, 0, 0, 0, 1, NULL, 0, 0), -- Bulwark of Smouldering Steel (flex)
(@HEALER, 5, 50404, 0, 0, 0, 1, NULL, 0, 0), -- Ashen Band of Endless Wisdom
(@HEALER, 6, 50718, 0, 0, 0, 1, NULL, 0, 0), -- Royal Crimson Cloak
(@HEALER, 7, 50766, 0, 0, 0, 1, NULL, 0, 0); -- Crimson Acolyte Gloves

-- Tank presets (Prot Warrior/Prot Paladin/Blood DK/Bear)
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`) VALUES
(@TANK, 0, 50738, 0, 0, 0, 1, NULL, 0, 0), -- Mithrios, Bronzebeard's Legacy
(@TANK, 1, 50364, 0, 0, 0, 1, NULL, 0, 0), -- Unidentifiable Organ
(@TANK, 2, 50356, 0, 0, 0, 1, NULL, 0, 0), -- Corroded Skeleton Key
(@TANK, 3, 50401, 0, 0, 0, 1, NULL, 0, 0), -- Ashen Band of Endless Courage
(@TANK, 4, 50729, 0, 0, 0, 1, NULL, 0, 0), -- Icecrown Glacial Wall
(@TANK, 5, 50611, 0, 0, 0, 1, NULL, 0, 0), -- Soulcleave Pendant
(@TANK, 6, 50775, 0, 0, 0, 1, NULL, 0, 0), -- Corrupted Silverplate Leggings
(@TANK, 7, 50712, 0, 0, 0, 1, NULL, 0, 0); -- Sanctified Ymirjar Lord's Greathelm

-- Ranged physical presets (Hunter only focus)
INSERT INTO npc_vendor (`entry`,`slot`,`item`,`maxcount`,`incrtime`,`ExtendedCost`,`type`,`BonusListIDs`,`PlayerConditionID`,`IgnoreFiltering`) VALUES
(@RANGED, 0, 50733, 0, 0, 0, 1, NULL, 0, 0), -- Fal'inrush, Defender of Quel'thalas
(@RANGED, 1, 50362, 0, 0, 0, 1, NULL, 0, 0), -- Deathbringer's Will
(@RANGED, 2, 50340, 0, 0, 0, 1, NULL, 0, 0), -- Muradin's Spyglass
(@RANGED, 3, 50735, 0, 0, 0, 1, NULL, 0, 0), -- Oathbinder, Charge of the Ranger-General
(@RANGED, 4, 50777, 0, 0, 0, 1, NULL, 0, 0), -- Frostbrood Sapphire Ring
(@RANGED, 5, 50774, 0, 0, 0, 1, NULL, 0, 0), -- Sharpened Twilight Scale
(@RANGED, 6, 50653, 0, 0, 0, 1, NULL, 0, 0), -- Leggings of Northern Lights
(@RANGED, 7, 50646, 0, 0, 0, 1, NULL, 0, 0); -- Frostbinder's Shredded Cape
