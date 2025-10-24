//! Jump marks system for navigation bookmarks
//!
//! This module is only available on native targets (depends on persistent history).

#![cfg(feature = "native")]

use crate::history::{History, PersistedMark};
use crate::types::Mark;

const LABELS: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "7", "8", "9",
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
];

pub struct JumpMarks {
    marks: Vec<Mark>,
    cursor: usize,
    history: History,
}

impl JumpMarks {
    pub fn new(history: History) -> Self {
        Self {
            marks: Vec::new(),
            cursor: 0,
            history,
        }
    }

    pub async fn load_from_persistence(&mut self) {
        let persisted = self.history.list_marks().await;
        self.marks = persisted
            .into_iter()
            .map(|p| Mark {
                label: p.label,
                pane: p.pane,
                height: p.height,
                tx_hash: p.tx,
                when_ms: p.when_ms,
                pinned: p.pinned,
            })
            .collect();
    }

    pub fn list(&self) -> Vec<Mark> {
        let mut sorted = self.marks.clone();
        sorted.sort_by(|a, b| b.when_ms.cmp(&a.when_ms)); // Newest first
        sorted
    }

    pub fn get_by_label(&self, label: &str) -> Option<&Mark> {
        self.marks.iter().find(|m| m.label == label)
    }

    pub fn next_auto_label(&self) -> String {
        for &label in LABELS {
            if !self.marks.iter().any(|m| m.label == label) {
                return label.to_string();
            }
        }
        // If all labels taken, reuse oldest
        self.marks
            .iter()
            .min_by_key(|m| m.when_ms)
            .map(|m| m.label.clone())
            .unwrap_or_else(|| "a".to_string())
    }

    pub async fn add_or_replace(&mut self, label: String, pane: u8, height: Option<u64>, tx_hash: Option<String>) {
        let now = chrono::Utc::now().timestamp_millis();

        // Preserve pinned status if updating existing mark
        let pinned = self.marks.iter()
            .find(|m| m.label == label)
            .map(|m| m.pinned)
            .unwrap_or(false);

        let mark = Mark {
            label: label.clone(),
            pane,
            height,
            tx_hash: tx_hash.clone(),
            when_ms: now,
            pinned,
        };

        // Update or add
        if let Some(pos) = self.marks.iter().position(|m| m.label == label) {
            self.marks[pos] = mark;
        } else {
            self.marks.push(mark);
        }

        self.cursor = 0;

        // Write-through to persistence
        let persisted = PersistedMark {
            label,
            pane,
            height,
            tx: tx_hash,
            when_ms: now,
            pinned,
        };
        self.history.put_mark(persisted).await;
    }

    pub async fn remove_by_label(&mut self, label: &str) {
        self.marks.retain(|m| m.label != label);
        if self.cursor >= self.marks.len() && self.cursor > 0 {
            self.cursor = self.marks.len() - 1;
        }
        self.history.del_mark(label.to_string()).await;
    }

    pub fn next(&mut self) -> Option<Mark> {
        let list = self.list();
        if list.is_empty() {
            return None;
        }
        self.cursor = (self.cursor + 1) % list.len();
        Some(list[self.cursor].clone())
    }

    pub fn prev(&mut self) -> Option<Mark> {
        let list = self.list();
        if list.is_empty() {
            return None;
        }
        self.cursor = if self.cursor == 0 {
            list.len() - 1
        } else {
            self.cursor - 1
        };
        Some(list[self.cursor].clone())
    }

    /// Find a mark by context (pane, height, tx_hash)
    /// Used to check if current context already has a mark
    pub fn find_by_context(&self, pane: u8, height: Option<u64>, tx_hash: Option<&str>) -> Option<String> {
        self.marks.iter()
            .find(|m| {
                // Match by tx_hash if present (most specific)
                if let Some(hash) = tx_hash {
                    return m.tx_hash.as_deref() == Some(hash);
                }
                // Otherwise match by height + pane if height present
                if let Some(h) = height {
                    return m.height == Some(h) && m.pane == pane && m.tx_hash.is_none();
                }
                // Otherwise match by pane only
                m.pane == pane && m.height.is_none() && m.tx_hash.is_none()
            })
            .map(|m| m.label.clone())
    }

    /// Toggle pin status of a mark
    pub async fn toggle_pin(&mut self, label: &str) {
        if let Some(mark) = self.marks.iter_mut().find(|m| m.label == label) {
            mark.pinned = !mark.pinned;
            self.history.set_mark_pinned(label.to_string(), mark.pinned).await;
        }
    }

    /// Set pin status of a mark explicitly
    pub async fn set_pinned(&mut self, label: &str, pinned: bool) {
        if let Some(mark) = self.marks.iter_mut().find(|m| m.label == label) {
            mark.pinned = pinned;
            self.history.set_mark_pinned(label.to_string(), pinned).await;
        }
    }
}
