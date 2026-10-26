use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GameEvent {
    pub event_id: String,
    pub player_id: String,
    pub kind: EventKind,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    AssetSubmitted { asset_id: String },
    LiveEventStarted { session_id: String },
    ModerationFlagged { asset_id: String },
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Decision {
    pub event_id: String,
    pub destination: String,
    pub subject: String,
}

pub fn route(event: &GameEvent) -> Decision {
    let (destination, subject) = match &event.kind {
        EventKind::AssetSubmitted { asset_id } => ("player-assets", asset_id),
        EventKind::LiveEventStarted { session_id } => ("live-events", session_id),
        EventKind::ModerationFlagged { asset_id } => ("moderation-review", asset_id),
    };
    Decision {
        event_id: event.event_id.clone(),
        destination: destination.into(),
        subject: format!("{}:{subject}", event.player_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flagged_asset_goes_to_moderation() {
        let event: GameEvent = serde_json::from_str(
            r#"{"event_id":"evt-12","player_id":"p-4","kind":{"type":"moderation_flagged","asset_id":"skin-9"}}"#
        ).unwrap();
        assert_eq!(route(&event), Decision {
            event_id: "evt-12".into(),
            destination: "moderation-review".into(),
            subject: "p-4:skin-9".into(),
        });
    }
}
