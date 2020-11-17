#[derive(Debug)]
pub struct Experiment {
    pub id: i64,
    pub cluster_size: i16,
    pub lr: f64,
    pub batch_size: i16,
    pub test_every: i16,
    pub update_every: i16,
    pub group_size: i16,
    pub history_length: i16,
    pub dropout: f64,
    pub adversarial: bool,
    pub krum: bool
}

impl Experiment {
    pub fn create_table() -> String {
        "
        CREATE TABLE IF NOT EXISTS experiments (
            id              INTEGER PRIMARY KEY,
            cluster_size    INTEGER,
            lr              FLOAT,
            batch_size      INTEGER,
            test_every      INTEGER,
            update_every    INTEGER,
            group_size      INTEGER,
            history_length  INTEGER,
            dropout         FLOAT,
            adversarial     BOOLEAN,
            krum            BOOLEAN
        );
        ".to_string()
    }

    pub fn insert_into() -> String {
        "INSERT INTO experiments (cluster_size, lr, batch_size, test_every, update_every, \
        group_size, history_length, dropout, adversarial, krum) \
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);".to_string()
    }
}


#[derive(Debug)]
pub struct Evaluation {
    pub id: i64,
    pub experiment_id: i64,
    pub timestamp: i64,
    pub step: i16,
    pub metric: String,
    pub value: f64
}

impl Evaluation {
    pub fn create_table() -> String {
        "
        CREATE TABLE IF NOT EXISTS evaluations (
            id              INTEGER PRIMARY KEY,
            experiment_id   INTEGER,
            timestamp       LONG,
            step            INTEGER,
            metric          VARCHAR,
            value           FLOAT,
            FOREIGN KEY(experiment_id) REFERENCES experiments(id)
        )
        ".to_string()
    }

    pub fn insert_into() -> String {
        "INSERT INTO evaluations (experiment_id, timestamp, step, metric, value) \
        VALUES (?1, ?2, ?3, ?4, ?5);".to_string()
    }
}