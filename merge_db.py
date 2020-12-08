import sqlite3
import pandas as pd
import os, sys
from typing import Tuple


DB_DIR = "./results/"


def load_db(name) -> Tuple[pd.DataFrame, pd.DataFrame]:
    conn = sqlite3.connect(name)
    return pd.read_sql("select * from experiments", conn), pd.read_sql("select * from main.evaluations", conn)


def add_rank(df: pd.DataFrame, rank: int) -> pd.DataFrame:
    df["rank"] = rank
    return df


def append_to_global(df: pd.DataFrame, global_con: sqlite3.Connection, experiments: bool):
    df.to_sql(("experiments" if experiments else "evaluations"), con=global_con, if_exists="append")


def main():
    global_db = sys.argv[1]
    con = sqlite3.connect(global_db)
    for db in sorted(os.listdir(DB_DIR)):
        if db.endswith(".db"):
            print(f"merge DB: {db}")
            rank = int(db.split(".")[1])
            df_exp, df_eva = load_db(os.path.join(DB_DIR, db))
            df_exp = add_rank(df_exp, rank)
            df_eva = add_rank(df_eva, rank)
            append_to_global(df_exp, con, True)
            append_to_global(df_eva, con, False)


if __name__ == "__main__":
    main()
