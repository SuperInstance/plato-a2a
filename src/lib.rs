use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Agent Card ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Available,
    Busy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub model: String,
    pub capabilities: Vec<String>,
    pub endpoint: String,
    pub status: AgentStatus,
    pub last_seen: u64,
}

impl AgentCard {
    pub fn new(name: &str, model: &str, capabilities: Vec<&str>, endpoint: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            model: model.to_string(),
            capabilities: capabilities.into_iter().map(String::from).collect(),
            endpoint: endpoint.to_string(),
            status: AgentStatus::Available,
            last_seen: now_ms(),
        }
    }
}

// ── Task Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskResultStatus {
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOffer {
    pub task_id: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub deadline_ms: u64,
    pub reward: String,
}

impl TaskOffer {
    pub fn new(description: &str, requirements: Vec<&str>) -> Self {
        Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            requirements: requirements.into_iter().map(String::from).collect(),
            deadline_ms: 30_000,
            reward: "placeholder".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAccept {
    pub task_id: String,
    pub agent_id: String,
    pub estimated_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub status: TaskResultStatus,
    pub output: String,
}

// ── Wire Message ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Discover,
    Advertise,
    TaskOffer,
    TaskAccept,
    TaskResult,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub from: String,
    pub to: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: u64,
    pub signature: String,
}

// ── Protocol ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct A2AProtocol {
    pub agent_id: String,
}

impl A2AProtocol {
    pub fn new() -> Self {
        Self {
            agent_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    fn make_message(
        &self,
        to: &str,
        msg_type: MessageType,
        payload: serde_json::Value,
    ) -> A2AMessage {
        A2AMessage {
            from: self.agent_id.clone(),
            to: to.to_string(),
            message_type: msg_type,
            payload,
            timestamp: now_ms(),
            signature: format!("sig:placeholder:{}", self.agent_id),
        }
    }

    pub fn advertise(&self, card: &AgentCard) -> String {
        let msg = self.make_message(
            "broadcast",
            MessageType::Advertise,
            serde_json::to_value(card).unwrap(),
        );
        serde_json::to_string(&msg).unwrap()
    }

    pub fn discover(&self) -> String {
        let msg = self.make_message(
            "broadcast",
            MessageType::Discover,
            serde_json::json!({"query": "all"}),
        );
        serde_json::to_string(&msg).unwrap()
    }

    pub fn heartbeat(&self) -> String {
        let msg = self.make_message(
            "broadcast",
            MessageType::Heartbeat,
            serde_json::json!({"status": "alive"}),
        );
        serde_json::to_string(&msg).unwrap()
    }

    pub fn offer_task(&self, task: &TaskOffer) -> String {
        let msg = self.make_message(
            "broadcast",
            MessageType::TaskOffer,
            serde_json::to_value(task).unwrap(),
        );
        serde_json::to_string(&msg).unwrap()
    }

    pub fn accept_task(&self, accept: &TaskAccept) -> String {
        let msg = self.make_message(
            &accept.agent_id,
            MessageType::TaskAccept,
            serde_json::to_value(accept).unwrap(),
        );
        serde_json::to_string(&msg).unwrap()
    }

    pub fn deliver_result(&self, result: &TaskResult) -> String {
        let msg = self.make_message(
            &result.agent_id,
            MessageType::TaskResult,
            serde_json::to_value(result).unwrap(),
        );
        serde_json::to_string(&msg).unwrap()
    }

    pub fn parse(&self, raw: &str) -> Result<A2AMessage, String> {
        serde_json::from_str(raw).map_err(|e| format!("parse error: {}", e))
    }
}

// ── Capability Matcher ──────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct CapabilityMatcher;

impl CapabilityMatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn score(&self, card: &AgentCard, requirements: &[&str]) -> f64 {
        if requirements.is_empty() {
            return 0.0;
        }
        let matched = requirements
            .iter()
            .filter(|req| card.capabilities.iter().any(|c| c == *req))
            .count();
        matched as f64 / requirements.len() as f64
    }
}

// ── Agent Registry ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, AgentCard>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, card: AgentCard) {
        self.agents.insert(card.id.clone(), card);
    }

    pub fn find_by_capability(&self, cap: &str) -> Vec<&AgentCard> {
        self.agents
            .values()
            .filter(|c| c.capabilities.iter().any(|c2| c2 == cap))
            .collect()
    }

    pub fn find_best_for(&self, requirements: &[&str]) -> Option<&AgentCard> {
        let matcher = CapabilityMatcher::new();
        self.agents
            .values()
            .filter(|c| c.status == AgentStatus::Available)
            .max_by(|a, b| {
                matcher
                    .score(a, requirements)
                    .partial_cmp(&matcher.score(b, requirements))
                    .unwrap()
            })
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

// ── Task Negotiation ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TaskNegotiation {
    pub protocol: A2AProtocol,
}

impl TaskNegotiation {
    pub fn new(protocol: A2AProtocol) -> Self {
        Self { protocol }
    }

    pub fn offer(&self, task: &TaskOffer) -> String {
        self.protocol.offer_task(task)
    }

    pub fn accept(&self, accept: &TaskAccept) -> String {
        self.protocol.accept_task(accept)
    }

    pub fn deliver(&self, result: &TaskResult) -> String {
        self.protocol.deliver_result(result)
    }

    pub fn parse(&self, raw: &str) -> Result<A2AMessage, String> {
        self.protocol.parse(raw)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card(name: &str, caps: Vec<&str>) -> AgentCard {
        AgentCard::new(name, "test-model", caps, "tcp://localhost:9000")
    }

    #[test]
    fn advertise_produces_valid_json() {
        let proto = A2AProtocol::new();
        let card = make_card("agent-1", vec!["codegen", "review"]);
        let json = proto.advertise(&card);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["message_type"], "advertise");
        assert!(parsed["payload"]["name"].is_string());
    }

    #[test]
    fn parse_handles_all_message_types() {
        let proto = A2AProtocol::new();
        let types = vec![
            proto.discover(),
            proto.advertise(&make_card("a", vec![])),
            proto.heartbeat(),
            proto.offer_task(&TaskOffer::new("t", vec![])),
            proto.accept_task(&TaskAccept {
                task_id: "t1".into(),
                agent_id: "a1".into(),
                estimated_time_ms: 1000,
            }),
            proto.deliver_result(&TaskResult {
                task_id: "t1".into(),
                agent_id: "a1".into(),
                status: TaskResultStatus::Success,
                output: "done".into(),
            }),
        ];
        let expected = vec![
            MessageType::Discover,
            MessageType::Advertise,
            MessageType::Heartbeat,
            MessageType::TaskOffer,
            MessageType::TaskAccept,
            MessageType::TaskResult,
        ];
        for (raw, exp) in types.iter().zip(expected.iter()) {
            let msg = proto.parse(raw).unwrap();
            assert_eq!(msg.message_type, *exp);
        }
    }

    #[test]
    fn parse_rejects_malformed_json() {
        let proto = A2AProtocol::new();
        assert!(proto.parse("{not valid}").is_err());
        assert!(proto.parse("").is_err());
        assert!(proto.parse("null").is_err());
    }

    #[test]
    fn register_stores_agent_card() {
        let mut reg = AgentRegistry::new();
        let card = make_card("a1", vec!["codegen"]);
        let id = card.id.clone();
        reg.register(card);
        assert_eq!(reg.len(), 1);
        assert!(reg.agents.contains_key(&id));
    }

    #[test]
    fn find_by_capability_returns_matching_agents() {
        let mut reg = AgentRegistry::new();
        reg.register(make_card("a1", vec!["codegen", "review"]));
        reg.register(make_card("a2", vec!["testing"]));
        reg.register(make_card("a3", vec!["codegen"]));
        let found = reg.find_by_capability("codegen");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn find_best_for_ranks_by_capability_overlap() {
        let mut reg = AgentRegistry::new();
        reg.register(make_card("weak", vec!["codegen"]));
        reg.register(make_card("strong", vec!["codegen", "review", "testing"]));
        let best = reg.find_best_for(&["codegen", "review"]);
        assert!(best.is_some());
        assert_eq!(best.unwrap().name, "strong");
    }

    #[test]
    fn matcher_scores_higher_for_more_matches() {
        let matcher = CapabilityMatcher::new();
        let weak = make_card("w", vec!["codegen"]);
        let strong = make_card("s", vec!["codegen", "review"]);
        let reqs = vec!["codegen", "review"];
        assert!(matcher.score(&strong, &reqs) > matcher.score(&weak, &reqs));
    }

    #[test]
    fn task_offer_accept_result_round_trip() {
        let proto = A2AProtocol::new();
        let nego = TaskNegotiation::new(proto);
        let task = TaskOffer::new("build crate", vec!["rust", "codegen"]);
        let offer_raw = nego.offer(&task);
        let offer_msg = nego.parse(&offer_raw).unwrap();
        assert_eq!(offer_msg.message_type, MessageType::TaskOffer);

        let accept = TaskAccept {
            task_id: task.task_id.clone(),
            agent_id: "agent-x".into(),
            estimated_time_ms: 5000,
        };
        let accept_raw = nego.accept(&accept);
        let accept_msg = nego.parse(&accept_raw).unwrap();
        assert_eq!(accept_msg.message_type, MessageType::TaskAccept);

        let result = TaskResult {
            task_id: task.task_id.clone(),
            agent_id: "agent-x".into(),
            status: TaskResultStatus::Success,
            output: "built successfully".into(),
        };
        let result_raw = nego.deliver(&result);
        let result_msg = nego.parse(&result_raw).unwrap();
        assert_eq!(result_msg.message_type, MessageType::TaskResult);
    }

    #[test]
    fn signature_placeholder_present() {
        let proto = A2AProtocol::new();
        let raw = proto.heartbeat();
        let msg = proto.parse(&raw).unwrap();
        assert!(msg.signature.starts_with("sig:placeholder:"));
    }

    #[test]
    fn heartbeat_formats_correctly() {
        let proto = A2AProtocol::new();
        let raw = proto.heartbeat();
        let msg = proto.parse(&raw).unwrap();
        assert_eq!(msg.message_type, MessageType::Heartbeat);
        assert_eq!(msg.to, "broadcast");
        assert_eq!(msg.payload["status"], "alive");
    }

    #[test]
    fn multiple_agents_overlapping_capabilities_ranked() {
        let mut reg = AgentRegistry::new();
        reg.register(make_card("a1", vec!["codegen", "review", "testing", "deploy"]));
        reg.register(make_card("a2", vec!["codegen", "review"]));
        reg.register(make_card("a3", vec!["codegen"]));
        let best = reg.find_best_for(&["codegen", "review", "testing"]);
        assert_eq!(best.unwrap().name, "a1");
    }

    #[test]
    fn matcher_returns_zero_for_no_requirements() {
        let matcher = CapabilityMatcher::new();
        let card = make_card("a", vec!["codegen"]);
        assert_eq!(matcher.score(&card, &[]), 0.0);
    }

    #[test]
    fn find_best_for_returns_none_when_empty() {
        let reg = AgentRegistry::new();
        assert!(reg.find_best_for(&["codegen"]).is_none());
    }
}
