# Bruno collection

Collection path: `bruno-collection/`

## Usage

1. Open Bruno and import folder `bruno-collection/`.
2. Select environment `local` from `environments/local.bru`.
3. Run `auth/sign-in` and copy tokens into env vars:
   - `access_token`
   - `refresh_token`
4. Run protected endpoints.

## Notes

- Admin endpoints require admin role/permissions in JWT payload.
- `character_guid`, `item_entry`, and `account_id` are placeholders and should be changed per test data.
