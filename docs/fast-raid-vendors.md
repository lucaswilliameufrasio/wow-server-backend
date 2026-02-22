# Fast raid-prep vendors (AzerothCore)

This adds:
- 10 class vendors (one per class)
- 4 misc vendors (universal epics, consumables, gems/enchants, mounts/misc)

Location:
- Dalaran (`map=571`) around `x=5815`, `y=650`, `z=647`

## Apply in one command

```bash
make seed-fast-raid-vendors
```

Fixed ICC-style preset vendors:

```bash
make seed-spec-bis-vendors
```

Equivalent script:

```bash
./scripts/seed-fast-raid-vendors.sh
```

## Optional env overrides

```bash
export ACORE_REPO_DIR=/tmp/azerothcore-wotlk
export WORLD_DB_NAME=acore_world
export VENDOR_SQL_FILE=/path/to/fast_raid_vendors.sql
make seed-fast-raid-vendors
```

## What gets sold

- Class vendors: top epic class-usable gear/weapons from `item_template`
  - filter: `Quality >= 4`, `ItemLevel >= 245`, class mask-compatible
- Universal vendor: high ilvl epics for all classes
- Consumables vendor: items with names matching flask/potion/elixir/food/feast/bandage
- Gems/enchants vendor: gems + trade/enchant utility classes
- Mounts/misc vendor: convenience/misc categories

## Reload without full restart (GM)

```text
.reload creature_template
.reload creature
.reload npc_vendor
```

## Customize

Edit:
- `sql/fast_raid_vendors.sql`

Common tweaks:
- Move spawn coordinates (`@X`, `@Y`, `@Z`)
- Change ilvl threshold (e.g. `ItemLevel >= 264`)
- Change vendor item limits (`LIMIT 250`)
