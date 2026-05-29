# plato-a2a — Agent-to-Agent Wire Protocol

Serialize and deserialize messages for the Google A2A (Agent-to-Agent) protocol. Agent cards, task offers, accept/reject handshakes, result delivery — all the types you need for inter-agent communication.

**Part of the [Plato](https://github.com/SuperInstance/plato-shell) ecosystem.**

## What This Gives You

- **Agent cards** — identity, capabilities, endpoint, status (Available/Busy/Offline)
- **Task lifecycle** — offer → accept/reject → result (Success/Partial/Failed)
- **Typed artifacts** — structured results with MIME type and metadata
- **Serde serialization** — JSON wire format out of the box
- **UUID identifiers** — unique task and agent IDs

## Quick Start

```rust
use plato_a2a::{AgentCard, TaskOffer, TaskAccept, TaskResult, TaskResultStatus};

// Create an agent card
let card = AgentCard::new("analyst", "glm-5.1", vec!["code_generation"], "ws://localhost:9001");

// Offer a task
let offer = TaskOffer::new("Analyze dataset", vec!["python", "pandas"]);

// Accept it
let accept = TaskAccept::new(&offer.task_id, &card.id, 5000);

// Deliver results
let result = TaskResult::new(&offer.task_id, &card.id)
    .with_status(TaskResultStatus::Success)
    .with_artifact("analysis.json", "application/json", br#"{"score": 0.95}"#);
```

## API Reference

| Type | Description |
|------|-------------|
| `AgentCard` | Agent identity: name, model, capabilities, endpoint, status |
| `TaskOffer` | Task proposal with requirements and deadline |
| `TaskAccept` | Acceptance with estimated completion time |
| `TaskReject` | Rejection with reason |
| `TaskResult` | Delivery with status, artifacts, and metadata |
| `AgentStatus` | Available, Busy, Offline |
| `TaskResultStatus` | Success, Partial, Failed |

## How It Fits

Used by [plato-tick](https://github.com/SuperInstance/plato-tick) for inter-agent messaging and [plato-fleet](https://github.com/SuperInstance/plato-fleet) for agent discovery. Implements the A2A wire format so Plato agents can communicate with any A2A-compatible system.

## Installation

```toml
[dependencies]
plato-a2a = "0.1"
```

## License

MIT
