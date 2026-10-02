use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeKind {
    Dns,
    Tcp,
    Tls,
    Certificate,
    Auth,
    Folders,
    Smtp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeStatus {
    Ok,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeStep {
    pub kind: ProbeKind,
    pub status: ProbeStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ConnectProbe {
    pub steps: Vec<ProbeStep>,
}

impl ConnectProbe {
    #[must_use]
    pub fn success(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.status == ProbeStatus::Ok)
    }

    #[must_use]
    pub fn step(&self, kind: ProbeKind) -> Option<&ProbeStep> {
        self.steps.iter().find(|s| s.kind == kind)
    }

    pub fn push_ok(&mut self, kind: ProbeKind, detail: impl Into<String>) {
        self.steps.push(ProbeStep { kind, status: ProbeStatus::Ok, detail: detail.into() });
    }

    pub fn push_fail(&mut self, kind: ProbeKind, detail: impl Into<String>) {
        self.steps.push(ProbeStep { kind, status: ProbeStatus::Failed, detail: detail.into() });
    }

    pub fn skip_rest(&mut self, from: ProbeKind) {
        let order = [
            ProbeKind::Dns,
            ProbeKind::Tcp,
            ProbeKind::Tls,
            ProbeKind::Certificate,
            ProbeKind::Auth,
            ProbeKind::Folders,
        ];
        let start = order.iter().position(|k| *k == from).unwrap_or(0);
        for kind in order.into_iter().skip(start) {
            if self.step(kind).is_none() {
                self.steps.push(ProbeStep {
                    kind,
                    status: ProbeStatus::Skipped,
                    detail: String::new(),
                });
            }
        }
    }
}
