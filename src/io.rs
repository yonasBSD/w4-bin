use std::{cell::RefCell, sync::LazyLock};

use actix_web::web::Bytes;
use linked_hash_map::LinkedHashMap;
use parking_lot::RwLock;
// In rand 0.10, Alphanumeric is moved to rand::distr and sample_iter needs RngExt in scope
use rand::{RngExt, distr::Alphanumeric, rng};

pub type PasteStore = RwLock<LinkedHashMap<String, Bytes>>;

static BUFFER_SIZE: LazyLock<usize> =
    LazyLock::new(|| argh::from_env::<crate::BinArgs>().buffer_size);

/// Ensures `ENTRIES` is less than the size of `BIN_BUFFER_SIZE`. If it isn't then
/// `ENTRIES.len() - BIN_BUFFER_SIZE` elements will be popped off the front of the map.
///
/// During the purge, `ENTRIES` is locked and the current thread will block.
fn purge_old(entries: &mut LinkedHashMap<String, Bytes>) {
    if entries.len() > *BUFFER_SIZE {
        let to_remove = entries.len() - *BUFFER_SIZE;

        for _ in 0..to_remove {
            entries.pop_front();
        }
    }
}

/// Generates a 'pronounceable' random ID using gpw
pub fn generate_id() -> String {
    thread_local!(static KEYGEN: RefCell<gpw::PasswordGenerator> = RefCell::new(gpw::PasswordGenerator::default()));

    KEYGEN.with(|k| k.borrow_mut().next()).unwrap_or_else(|| {
        // Updated for rand 0.10 compatibility: RngExt trait must be in scope for sample_iter
        rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect()
    })
}

/// Stores a paste under the given id
pub fn store_paste(entries: &PasteStore, id: String, content: Bytes) {
    let mut entries = entries.write();

    purge_old(&mut entries);

    entries.insert(id, content);
}

/// Returns the paste with the given id
pub fn get_paste(entries: &PasteStore, id: &str) -> Option<Bytes> {
    entries.read().get(id).cloned()
}
