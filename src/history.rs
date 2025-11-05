//! Transaction history persistence and search
//!
//! Note: SQLite-based history is only available on native targets.
//! Web targets use an in-memory stub implementation.

use anyhow::Result;

#[cfg(feature = "native")]
use rusqlite::{params, Connection, Statement, ToSql};
#[cfg(feature = "native")]
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
#[cfg(feature = "native")]
use tokio::sync::oneshot;
#[cfg(feature = "native")]
use tokio::task::spawn_blocking;

#[derive(Clone, Debug)]
pub struct TxPersist {
    pub hash: String,
    #[allow(dead_code)]
    pub height: u64,
    pub signer: Option<String>,
    pub receiver: Option<String>,
    pub actions_json: Option<String>,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BlockPersist {
    pub height: u64,
    pub hash: String,
    pub ts_ms: i64,
    pub txs: Vec<TxPersist>,
}

#[derive(Clone, Debug)]
pub struct HistoryHit {
    pub hash: String,
    pub height: u64,
    pub ts_ms: i64,
    pub signer: Option<String>,
    pub receiver: Option<String>,
    pub methods: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PersistedMark {
    pub label: String,
    pub pane: u8,
    pub height: Option<u64>,
    pub tx: Option<String>,
    pub when_ms: i64,
    pub pinned: bool,
}

// Native-only History implementation using SQLite
#[cfg(feature = "native")]
enum HistoryMsg {
    Persist(BlockPersist),
    Search {
        query: String,
        limit: usize,
        resp: oneshot::Sender<Vec<HistoryHit>>,
    },
    GetTx {
        hash: String,
        resp: oneshot::Sender<Option<String>>,
    },
    ListMarks {
        resp: oneshot::Sender<Vec<PersistedMark>>,
    },
    PutMark {
        mark: PersistedMark,
        resp: oneshot::Sender<()>,
    },
    DelMark {
        label: String,
        resp: oneshot::Sender<()>,
    },
    SetMarkPinned {
        label: String,
        pinned: bool,
        resp: oneshot::Sender<()>,
    },
    #[allow(dead_code)]
    ClearMarks {
        resp: oneshot::Sender<()>,
    },
}

#[cfg(feature = "native")]
#[derive(Clone)]
pub struct History {
    tx: UnboundedSender<HistoryMsg>,
}

#[cfg(feature = "native")]
impl History {
    pub fn start(db_path: &str) -> Result<Self> {
        let (tx, mut rx) = unbounded_channel::<HistoryMsg>();
        let path = db_path.to_string();

        tokio::spawn(async move {
            // single worker connection off main thread
            let _ = spawn_blocking(move || -> Result<()> {
                let conn = Connection::open(path)?;
                // Enable WAL mode for concurrent read/write performance
                conn.pragma_update(None, "journal_mode", &"WAL")?;
                conn.pragma_update(None, "synchronous", &"NORMAL")?;
                // Set busy timeout to avoid immediate lock failures
                conn.pragma_update(None, "busy_timeout", &250)?;
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS blocks(
                        height INTEGER PRIMARY KEY,
                        hash   TEXT NOT NULL,
                        ts_ms  INTEGER NOT NULL,
                        tx_count INTEGER NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS txs(
                        hash     TEXT PRIMARY KEY,
                        height   INTEGER NOT NULL,
                        signer   TEXT,
                        receiver TEXT,
                        actions_json TEXT,
                        raw_json TEXT,
                        FOREIGN KEY(height) REFERENCES blocks(height) ON DELETE CASCADE
                    );
                    CREATE INDEX IF NOT EXISTS idx_txs_signer   ON txs(signer);
                    CREATE INDEX IF NOT EXISTS idx_txs_receiver ON txs(receiver);
                    CREATE INDEX IF NOT EXISTS idx_txs_height   ON txs(height);
                    CREATE INDEX IF NOT EXISTS idx_txs_hash     ON txs(hash);
                    CREATE INDEX IF NOT EXISTS idx_blocks_height ON blocks(height);
                    CREATE TABLE IF NOT EXISTS marks(
                        label    TEXT PRIMARY KEY,
                        pane     INTEGER NOT NULL,
                        height   INTEGER,
                        tx       TEXT,
                        when_ms  INTEGER NOT NULL,
                        pinned   INTEGER NOT NULL DEFAULT 0
                    );
                    CREATE INDEX IF NOT EXISTS idx_marks_pinned ON marks(pinned) WHERE pinned = 1;
                "#,
                )?;

                let mut stmt_block = conn.prepare(
                    "INSERT OR REPLACE INTO blocks(height,hash,ts_ms,tx_count) VALUES (?,?,?,?)",
                )?;
                let mut stmt_tx = conn.prepare(
                    "INSERT OR REPLACE INTO txs(hash,height,signer,receiver,actions_json,raw_json) VALUES (?,?,?,?,?,?)",
                )?;

                // Mark statements
                let mut stmt_mark_upsert = conn.prepare(
                    "INSERT OR REPLACE INTO marks(label,pane,height,tx,when_ms,pinned) VALUES (?,?,?,?,?,?)",
                )?;
                let mut stmt_mark_del = conn.prepare(
                    "DELETE FROM marks WHERE label = ?",
                )?;
                let mut stmt_mark_set_pinned = conn.prepare(
                    "UPDATE marks SET pinned = ? WHERE label = ?",
                )?;
                let mut stmt_mark_clear = conn.prepare(
                    "DELETE FROM marks",
                )?;

                while let Some(msg) = rx.blocking_recv() {
                    match msg {
                        HistoryMsg::Persist(b) => {
                            let txc = conn.unchecked_transaction()?;
                            stmt_block.execute(params![
                                b.height as i64,
                                b.hash,
                                b.ts_ms,
                                b.txs.len() as i64
                            ])?;
                            for t in b.txs {
                                stmt_tx.execute(params![
                                    t.hash,
                                    b.height as i64,
                                    t.signer,
                                    t.receiver,
                                    t.actions_json,
                                    t.raw_json
                                ])?;
                            }
                            txc.commit()?;
                        }
                        HistoryMsg::Search { query, limit, resp } => {
                            let hits = search_db(&conn, &query, limit).unwrap_or_default();
                            let _ = resp.send(hits);
                        }
                        HistoryMsg::GetTx { hash, resp } => {
                            let raw = get_tx_db(&conn, &hash).unwrap_or(None);
                            let _ = resp.send(raw);
                        }
                        HistoryMsg::ListMarks { resp } => {
                            let marks = list_marks_db(&conn).unwrap_or_default();
                            let _ = resp.send(marks);
                        }
                        HistoryMsg::PutMark { mark, resp } => {
                            let _ = put_mark_db(&conn, &mut stmt_mark_upsert, &mark);
                            let _ = resp.send(());
                        }
                        HistoryMsg::DelMark { label, resp } => {
                            let _ = del_mark_db(&conn, &mut stmt_mark_del, &label);
                            let _ = resp.send(());
                        }
                        HistoryMsg::SetMarkPinned { label, pinned, resp } => {
                            let _ = set_mark_pinned_db(&conn, &mut stmt_mark_set_pinned, &label, pinned);
                            let _ = resp.send(());
                        }
                        HistoryMsg::ClearMarks { resp } => {
                            let _ = clear_marks_db(&conn, &mut stmt_mark_clear);
                            let _ = resp.send(());
                        }
                    }
                }
                Ok(())
            })
            .await;
        });

        Ok(Self { tx })
    }

    pub fn persist_block(&self, b: BlockPersist) {
        let _ = self.tx.send(HistoryMsg::Persist(b));
    }

    pub async fn search(&self, query: String, limit: usize) -> Vec<HistoryHit> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.tx.send(HistoryMsg::Search { query, limit, resp: resp_tx }).is_err() {
            return Vec::new();
        }
        resp_rx.await.unwrap_or_default()
    }

    pub async fn get_tx(&self, hash: String) -> Option<String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.tx.send(HistoryMsg::GetTx { hash, resp: resp_tx }).is_err() {
            return None;
        }
        resp_rx.await.ok().flatten()
    }

    pub async fn list_marks(&self) -> Vec<PersistedMark> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.tx.send(HistoryMsg::ListMarks { resp: resp_tx }).is_err() {
            return Vec::new();
        }
        resp_rx.await.unwrap_or_default()
    }

    pub async fn put_mark(&self, mark: PersistedMark) {
        let (resp_tx, resp_rx) = oneshot::channel();
        let _ = self.tx.send(HistoryMsg::PutMark { mark, resp: resp_tx });
        let _ = resp_rx.await;
    }

    pub async fn del_mark(&self, label: String) {
        let (resp_tx, resp_rx) = oneshot::channel();
        let _ = self.tx.send(HistoryMsg::DelMark { label, resp: resp_tx });
        let _ = resp_rx.await;
    }

    pub async fn set_mark_pinned(&self, label: String, pinned: bool) {
        let (resp_tx, resp_rx) = oneshot::channel();
        let _ = self.tx.send(HistoryMsg::SetMarkPinned { label, pinned, resp: resp_tx });
        let _ = resp_rx.await;
    }

    #[allow(dead_code)]
    pub async fn clear_marks(&self) {
        let (resp_tx, resp_rx) = oneshot::channel();
        let _ = self.tx.send(HistoryMsg::ClearMarks { resp: resp_tx });
        let _ = resp_rx.await;
    }
}

// Search query parser: signer: receiver: acct: method: action: from: to: hash: + free text
#[cfg(feature = "native")]
struct SearchQuery {
    signer: Vec<String>,
    receiver: Vec<String>,
    acct: Vec<String>,
    method: Vec<String>,
    action: Vec<String>,
    hash: Vec<String>,
    from_height: Option<i64>,
    to_height: Option<i64>,
    free: Vec<String>,
}

#[cfg(feature = "native")]
fn parse_search_query(q: &str) -> SearchQuery {
    let mut sq = SearchQuery {
        signer: vec![],
        receiver: vec![],
        acct: vec![],
        method: vec![],
        action: vec![],
        hash: vec![],
        from_height: None,
        to_height: None,
        free: vec![],
    };

    for tok in q.split_whitespace() {
        if let Some((k, v)) = tok.split_once(':') {
            let k = k.to_lowercase();
            let v = v.to_lowercase();
            match k.as_str() {
                "signer" => sq.signer.push(v),
                "receiver" | "rcv" => sq.receiver.push(v),
                "acct" | "account" => sq.acct.push(v),
                "method" => sq.method.push(v),
                "action" => sq.action.push(v),
                "hash" => sq.hash.push(v),
                "from" => sq.from_height = v.parse().ok(),
                "to" => sq.to_height = v.parse().ok(),
                _ => sq.free.push(tok.to_lowercase()),
            }
        } else if !tok.is_empty() {
            sq.free.push(tok.to_lowercase());
        }
    }
    sq
}

#[cfg(feature = "native")]
fn search_db(conn: &Connection, query: &str, limit: usize) -> Result<Vec<HistoryHit>> {
    let sq = parse_search_query(query);
    let mut sql = String::from("SELECT t.hash, t.height, b.ts_ms, t.signer, t.receiver, t.actions_json FROM txs t JOIN blocks b ON b.height = t.height");
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

    // acct: signer OR receiver
    if !sq.acct.is_empty() {
        for a in &sq.acct {
            where_clauses.push("(LOWER(t.signer) LIKE ? OR LOWER(t.receiver) LIKE ?)".to_string());
            let pattern = format!("%{}%", a);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }
    }

    // signer
    if !sq.signer.is_empty() {
        let clause = format!("({})", vec!["LOWER(t.signer) LIKE ?"; sq.signer.len()].join(" OR "));
        where_clauses.push(clause);
        for s in &sq.signer {
            params_vec.push(Box::new(format!("%{}%", s)));
        }
    }

    // receiver
    if !sq.receiver.is_empty() {
        let clause = format!("({})", vec!["LOWER(t.receiver) LIKE ?"; sq.receiver.len()].join(" OR "));
        where_clauses.push(clause);
        for r in &sq.receiver {
            params_vec.push(Box::new(format!("%{}%", r)));
        }
    }

    // hash
    if !sq.hash.is_empty() {
        let clause = format!("({})", vec!["LOWER(t.hash) = ?"; sq.hash.len()].join(" OR "));
        where_clauses.push(clause);
        for h in &sq.hash {
            params_vec.push(Box::new(h.clone()));
        }
    }

    // height range
    if let Some(from_h) = sq.from_height {
        where_clauses.push("t.height >= ?".to_string());
        params_vec.push(Box::new(from_h));
    }
    if let Some(to_h) = sq.to_height {
        where_clauses.push("t.height <= ?".to_string());
        params_vec.push(Box::new(to_h));
    }

    // method/action: LIKE on actions_json
    if !sq.method.is_empty() {
        let clause = format!("({})", vec!["LOWER(t.actions_json) LIKE ?"; sq.method.len()].join(" OR "));
        where_clauses.push(clause);
        for m in &sq.method {
            params_vec.push(Box::new(format!("%{}%", m)));
        }
    }
    if !sq.action.is_empty() {
        let clause = format!("({})", vec!["LOWER(t.actions_json) LIKE ?"; sq.action.len()].join(" OR "));
        where_clauses.push(clause);
        for a in &sq.action {
            params_vec.push(Box::new(format!("%{}%", a)));
        }
    }

    // free text: signer/receiver/hash/actions_json
    if !sq.free.is_empty() {
        let clause = format!("({})", vec!["(LOWER(t.signer)||' '||LOWER(t.receiver)||' '||LOWER(t.hash)||' '||LOWER(t.actions_json)) LIKE ?"; sq.free.len()].join(" AND "));
        where_clauses.push(clause);
        for f in &sq.free {
            params_vec.push(Box::new(format!("%{}%", f)));
        }
    }

    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }

    sql.push_str(" ORDER BY t.height DESC, t.hash LIMIT ?");
    params_vec.push(Box::new(std::cmp::min(limit, 500) as i64));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let actions_json: Option<String> = row.get(5)?;
        let methods = actions_json.as_ref().map(|aj| summarize_methods(aj));

        Ok(HistoryHit {
            hash: row.get(0)?,
            height: row.get::<_, i64>(1)? as u64,
            ts_ms: row.get(2)?,
            signer: row.get(3)?,
            receiver: row.get(4)?,
            methods,
        })
    })?;

    let mut hits = Vec::new();
    for r in rows {
        if let Ok(hit) = r {
            hits.push(hit);
        }
    }
    Ok(hits)
}

#[cfg(feature = "native")]
fn get_tx_db(conn: &Connection, hash: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT raw_json FROM txs WHERE hash = ?")?;
    let mut rows = stmt.query(params![hash])?;
    if let Some(row) = rows.next()? {
        let raw: Option<String> = row.get(0)?;
        return Ok(raw);
    }
    Ok(None)
}

#[cfg(feature = "native")]
fn summarize_methods(actions_json: &str) -> String {
    if let Ok(actions) = serde_json::from_str::<Vec<serde_json::Value>>(actions_json) {
        let mut methods = Vec::new();
        for a in actions {
            if let Some(fc) = a.get("FunctionCall") {
                if let Some(method) = fc.get("method_name").and_then(|m| m.as_str()) {
                    methods.push(method.to_string());
                }
            } else if let Some(obj) = a.as_object() {
                if let Some(action_type) = obj.keys().next() {
                    methods.push(action_type.to_string());
                }
            }
        }
        methods.join(", ")
    } else {
        String::new()
    }
}

#[cfg(feature = "native")]
fn list_marks_db(conn: &Connection) -> Result<Vec<PersistedMark>> {
    let mut stmt = conn.prepare("SELECT label, pane, height, tx, when_ms, pinned FROM marks ORDER BY when_ms DESC")?;
    let mut rows = stmt.query([])?;
    let mut marks = Vec::new();
    while let Some(row) = rows.next()? {
        marks.push(PersistedMark {
            label: row.get(0)?,
            pane: row.get(1)?,
            height: row.get(2)?,
            tx: row.get(3)?,
            when_ms: row.get(4)?,
            pinned: row.get::<_, i64>(5)? != 0,
        });
    }
    Ok(marks)
}

#[cfg(feature = "native")]
fn put_mark_db(_conn: &Connection, stmt: &mut Statement, mark: &PersistedMark) -> Result<()> {
    stmt.execute(params![
        &mark.label,
        mark.pane,
        mark.height.map(|h| h as i64),
        &mark.tx,
        mark.when_ms,
        mark.pinned as i64,
    ])?;
    Ok(())
}

#[cfg(feature = "native")]
fn del_mark_db(_conn: &Connection, stmt: &mut Statement, label: &str) -> Result<()> {
    stmt.execute(params![label])?;
    Ok(())
}

#[cfg(feature = "native")]
fn set_mark_pinned_db(_conn: &Connection, stmt: &mut Statement, label: &str, pinned: bool) -> Result<()> {
    stmt.execute(params![pinned as i64, label])?;
    Ok(())
}

#[cfg(feature = "native")]
fn clear_marks_db(_conn: &Connection, stmt: &mut Statement) -> Result<()> {
    stmt.execute([])?;
    Ok(())
}

// Web stub implementation (in-memory only, no persistence)
#[cfg(not(feature = "native"))]
#[derive(Clone)]
pub struct History;

#[cfg(not(feature = "native"))]
impl History {
    pub fn start(_db_path: &str) -> Result<Self> {
        Ok(History)
    }

    pub fn persist(&self, _block: BlockPersist) {}

    pub async fn search(&self, _query: &str, _limit: usize) -> Vec<HistoryHit> {
        Vec::new()
    }

    pub async fn get_tx(&self, _hash: &str) -> Option<String> {
        None
    }

    pub async fn list_marks(&self) -> Vec<PersistedMark> {
        Vec::new()
    }

    pub async fn put_mark(&self, _mark: PersistedMark) {}

    pub async fn del_mark(&self, _label: &str) {}

    pub async fn set_mark_pinned(&self, _label: &str, _pinned: bool) {}

    #[allow(dead_code)]
    pub async fn clear_marks(&self) {}
}
