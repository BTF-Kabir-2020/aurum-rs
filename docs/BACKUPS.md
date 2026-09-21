# Backups

Aurum ships a real backup/restore feature. Backups land in `./backups` by default.

## Commands

```
aurum backup create                 # default output under ./backups
aurum backup create --output ./backups/my-backup.db
aurum backup verify <file>
aurum backup restore <file>
```

## Create

`aurum backup create` must:

- produce a **consistent SQLite backup** (no corruption, no tearing)
- create the destination atomically where practical
- verify that the backup is readable
- **never silently overwrite** an existing backup
- print: backup path, backup size, database timestamp/version

## Verify

`aurum backup verify <file>` must:

- validate the file is a valid backup
- validate SQLite integrity
- report clear success/failure

## Restore

`aurum backup restore <file>` must be safe:

1. validate the backup
2. validate SQLite integrity of the backup
3. create a **safety copy** of the current DB
4. stop/lock application operations if necessary
5. restore
6. reopen
7. validate migrations/schema
8. report success/failure

Restore must **not** touch the current DB until the backup has passed validation. Restore input is scrubbed: sanitize filenames, reject path traversal, reject invalid backup files.

## Policy

We recommend a backup before:

- upgrade
- restore
- changing provider/storage configuration

`aurum backup create` doubles as the pre-upgrade step.

## Backup policy for tests

CI/integration tests must cover: create → verify → restore into a test environment → integrity check. Restore tests run against throwaway databases, never production data.