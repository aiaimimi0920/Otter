//! Verify the real Tile click route preserves opt-in form values per bubbling action.
use super::*;
use crate::tile_surface_value::tests::{fixture, presentation};
use serde_json::json;

#[test]
fn tile_click_only_attaches_form_values_to_opted_in_ancestor() {
    let f = fixture(json!({
        "id":"root","type":"column","events":{"click":"outer"},
        "props":{"includeSurfaceValues":true,"eventPayload":{"source":7}},
        "children":[
            {"id":"inner","type":"view","layout":{"width":64,"height":24,"minHeight":24},
             "events":{"click":"inner"},"props":{"eventPayload":{"keep":true}}},
            {"id":"field","type":"input","layout":{"width":64,"height":20},
             "props":{"value":"source🙂"}},
            {"id":"hidden","type":"input","props":{"visible":false,"value":"ignored"}}
        ]
    }));
    let p = presentation(&f);
    // The fixture's 2×2 output maps its upper-left sample into the first 64×24 row.
    let commands = prepare(&f.frame, &f.art, &p, [0, 0], Modifiers::default()).unwrap();
    let [inner, outer] = commands.as_slice() else {
        panic!("expected inner click followed by its ancestor");
    };
    let (CommandKind::Event(inner), CommandKind::Event(outer)) = (&inner.kind, &outer.kind) else {
        panic!("expected two discrete click events");
    };
    assert_eq!(inner["action"], "inner");
    assert_eq!(inner["payload"], json!({"keep":true}));
    assert_eq!(outer["action"], "outer");
    assert_eq!(
        outer["payload"],
        json!({"source":7,"surfaceValues":{"field":"source🙂"}})
    );
}
