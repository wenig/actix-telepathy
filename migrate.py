import sqlite3
import os
import sys

MIGRATIONS_DIR = "./examples/decentfl/migrations/"


def migrate(conn: sqlite3.Connection, name = ""):
    migration_table_exists = conn.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='migrations';").fetchone() is not None
    if migration_table_exists:
        last_successfull_migration = conn.execute("SELECT max(migration_id) FROM migrations WHERE success IS TRUE;").fetchone()[0] or -1
    else:
        last_successfull_migration = -1

    print(f"Database: {name}")
    print(f"Last migration: {last_successfull_migration}")

    for migration in sorted(os.listdir(MIGRATIONS_DIR)):
        if migration.endswith(".sql"):
            migration_id = int(migration.split("_")[0])
            if migration_id > last_successfull_migration:
                with open(os.path.join(MIGRATIONS_DIR, migration)) as f:
                    try:
                        c = conn.execute(f.read())
                        c.execute(f"INSERT INTO migrations (migration_id, success) VALUES ({migration_id}, TRUE);")
                        print(f"migrated to {migration_id}")
                        c.close()
                    except:
                        pass
        conn.commit()
    conn.close()


def bulk(directory):
    for db in os.listdir(directory):
        if db.endswith(".db"):
            conn = sqlite3.connect(os.path.join(directory, db))
            migrate(conn, db)


if __name__ == "__main__":
    db_path = sys.argv[1]
    if os.path.isdir(db_path):
        bulk(db_path)
    else:
        conn = sqlite3.connect(db_path)
        migrate(conn, db_path)
