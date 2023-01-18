use std::io::Result;
use std::path::PathBuf;

use helix_core::history::History;
use helix_core::Transaction;

pub fn serialize(session: &mut Session, editor: &Editor) -> Result<()> {
    todo!()
}

pub fn deserialize(session: &Session, editor: &mut Editor) -> Result<()> {
    todo!()
}

fn serialize_history(history: &History) -> Result<()> {
    todo!()
}

fn deserialize_history() -> Result<History> {
    todo!()
}

fn serialize_transaction(transaction: &Transaction) -> Result<()> {
    todo!()
}

fn deserialize_transaction() -> Result<Transaction> {
    todo!()
}
