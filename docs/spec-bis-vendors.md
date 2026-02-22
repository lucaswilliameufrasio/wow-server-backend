# Spec BiS preset vendors (fixed IDs)

This seeds deterministic ICC-style preset vendors using explicit item IDs (not auto-filter queries).

Command:

```bash
make seed-spec-bis-vendors
```

Source SQL:
- `sql/spec_bis_vendor_presets.sql`

Vendors spawned in Dalaran:
- Melee DPS preset
- Caster DPS preset
- Healer preset
- Tank preset
- Ranged physical preset

If you want exact guild-specific BiS, edit item IDs directly in:
- `sql/spec_bis_vendor_presets.sql`
