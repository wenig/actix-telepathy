mod models;

use rusqlite::{params, Connection, Result};
use models::*;
use std::time::{SystemTime, UNIX_EPOCH};


pub struct ScoreStorage {
    db_connection: Connection,
    experiment_id: Option<i64>
}


impl ScoreStorage {
    pub fn new(db_url: &str) -> Self {
        let conn = Connection::open(db_url).expect("Could not connect to DB");

        conn.execute(&Experiment::create_table(), params![])
            .expect("Could not create Experiment table");
        conn.execute(&Evaluation::create_table(), params![])
            .expect("Could not create Evaluation table");

        Self {
            db_connection: conn,
            experiment_id: None
        }
    }

    pub fn new_experiment(&mut self,
                          cluster_size: i16,
                          lr: f64,
                          batch_size: i16,
                          test_every: i16,
                          update_every: i16,
                          group_size: i16,
                          history_length: i16,
                          dropout: f64,
                          adversarial: bool,
                          krum: bool,
                          seed: i64,
                          centralized: bool) -> Result<()> {

        let new_experiment = Experiment {
            id: 0,
            cluster_size,
            lr,
            batch_size,
            test_every,
            update_every,
            group_size,
            history_length,
            dropout,
            adversarial,
            krum,
            seed,
            centralized
        };

        self.db_connection.execute(&Experiment::insert_into(),
                                   params![
                                   new_experiment.cluster_size,
                                   new_experiment.lr,
                                   new_experiment.batch_size,
                                   new_experiment.test_every,
                                   new_experiment.update_every,
                                   new_experiment.group_size,
                                   new_experiment.history_length,
                                   new_experiment.dropout,
                                   new_experiment.adversarial,
                                   new_experiment.krum,
                                   new_experiment.seed,
                                   new_experiment.centralized])?;
        let id = self.db_connection.last_insert_rowid();
        self.experiment_id = Some(id);
        Ok(())
    }

    pub fn add_result(&mut self, step: i16, metric: &str, value: f64) -> Result<()> {
        let new_result = Evaluation {
            id: 0,
            experiment_id: self.experiment_id.expect("You cannot add a result without creating an experiment!"),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).expect("Time traveled backwards").as_secs() as i64,
            step,
            metric: metric.to_string(),
            value
        };

        self.db_connection.execute(&Evaluation::insert_into(),
                                   params![
                                   new_result.experiment_id,
                                   new_result.timestamp,
                                   new_result.step,
                                   new_result.metric,
                                   new_result.value])?;
        Ok(())
    }
}