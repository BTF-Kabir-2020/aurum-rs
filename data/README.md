# data

Runtime directory. The SQLite database lives here (`aurum.db` by default).

- Mounted into Docker at `/data`
- Not committed to the repository
- Backed up with `aurum backup create`