use super::*;
use std::io::Cursor;
fn record() -> Record {
    Record {
        hwnd: 1,
        process_id: 2,
        thread_id: 3,
        process_created: 4,
        restoration: Restoration::Window(WindowState {
            rect: [0, 0, 32, 32],
            placement_flags: 0,
            show_command: 1,
            min_position: [0, 0],
            max_position: [0, 0],
            normal_position: [0, 0, 32, 32],
            extended_style: 0,
            layered: None,
            iconic: false,
            visible: true,
        }),
    }
}
fn window(record: &mut Record) -> &mut WindowState {
    let Restoration::Window(state) = &mut record.restoration else {
        panic!("window fixture")
    };
    state
}
#[test]
fn partial_popup_layer_round_trips_without_snapshot_and_rejects_missing_identity() {
    let mut record = record();
    let full = serde_json::to_value(&record).unwrap();
    assert_eq!(serde_json::from_value::<Record>(full).unwrap(), record);
    record.restoration = Restoration::PopupLayer(PopupLayer {
        owner: 8,
        root: 9,
        cookie: 10,
        epoch: 11,
        request: 12,
        instance: 13,
    });
    let mut bytes = Vec::new();
    let expected = request(Command::Register {
        id: 1,
        record: record.clone(),
    });
    write_request(&mut bytes, &expected).unwrap();
    assert_eq!(read_request(&mut &bytes[..]).unwrap(), Some(expected));
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["restoration"]["kind"], "popupLayer");
    for field in ["rect", "visible", "extendedStyle", "layered"] {
        assert!(value["restoration"]["state"].get(field).is_none());
        let mut mixed = value.clone();
        mixed["restoration"]["state"][field] = serde_json::json!(0);
        assert!(serde_json::from_value::<Record>(mixed).is_err(), "{field}");
    }
    let Restoration::PopupLayer(layer) = &mut record.restoration else {
        panic!("layer fixture")
    };
    layer.instance = 0;
    assert!(record.validate().is_err());
}
#[test]
fn restoration_kind_and_current_schema_are_required_no_legacy_default_snapshot() {
    let mut missing = serde_json::to_value(record()).unwrap();
    missing.as_object_mut().unwrap().remove("restoration");
    assert!(serde_json::from_value::<Record>(missing).is_err());
    let mut unknown = serde_json::to_value(record()).unwrap();
    unknown["restoration"]["kind"] = serde_json::json!("unknown");
    assert!(serde_json::from_value::<Record>(unknown).is_err());
    let mut old = request(Command::Register {
        id: 1,
        record: record(),
    });
    old.schema = 1;
    let mut bytes = Vec::new();
    assert!(write_request(&mut bytes, &old).is_err());
    assert!(bytes.is_empty());
    write_payload(&mut bytes, &old).unwrap();
    assert!(read_request(&mut &bytes[..]).is_err());
}
fn request(command: Command) -> Request {
    Request {
        schema: SCHEMA,
        session: [7; 16],
        sequence: 1,
        command,
    }
}
#[test]
fn register_retire_shutdown_and_ack_round_trip_without_trailing_consumption() {
    for command in [
        Command::Register {
            id: 1,
            record: record(),
        },
        Command::Retire { id: 1 },
        Command::Shutdown,
    ] {
        let expected = request(command);
        let mut bytes = Vec::new();
        write_request(&mut bytes, &expected).unwrap();
        let frame_len = bytes.len();
        bytes.extend_from_slice(b"trailing");
        let mut cursor = Cursor::new(bytes);
        assert_eq!(read_request(&mut cursor).unwrap(), Some(expected));
        assert_eq!(cursor.position() as usize, frame_len);
    }
    for status in [Status::Accepted, Status::Rejected] {
        let ack = Ack {
            schema: SCHEMA,
            session: [7; 16],
            sequence: 3,
            status,
        };
        let mut bytes = Vec::new();
        write_ack(&mut bytes, &ack).unwrap();
        assert_eq!(read_ack(&mut &bytes[..], [7; 16], 3).unwrap(), ack);
        assert!(read_ack(&mut &bytes[..], [8; 16], 3).is_err());
        assert!(read_ack(&mut &bytes[..], [7; 16], 2).is_err());
    }
}
#[test]
fn clean_eof_is_distinct_from_every_truncated_frame() {
    assert!(read_request(&mut &[][..]).unwrap().is_none());
    let mut bytes = Vec::new();
    write_request(
        &mut bytes,
        &request(Command::Register {
            id: 1,
            record: record(),
        }),
    )
    .unwrap();
    for end in 1..bytes.len() {
        assert!(read_request(&mut &bytes[..end]).is_err(), "prefix {end}");
    }
    assert!(read_ack(&mut &[][..], [7; 16], 1).is_err());
}
#[test]
fn oversize_zero_unknown_fields_and_invalid_json_are_rejected() {
    for length in [0, MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
        let header = length.to_le_bytes();
        assert!(read_request(&mut &header[..]).is_err());
    }
    let mut value = serde_json::to_value(request(Command::Shutdown)).unwrap();
    value["unexpected"] = serde_json::json!(true);
    let mut bytes = Vec::new();
    write_payload(&mut bytes, &value).unwrap();
    assert!(read_request(&mut &bytes[..]).is_err());
    assert!(read_request(&mut &[1, 0, 0, 0, b'{'][..]).is_err());
    let value = serde_json::json!({"schema":SCHEMA,"session":[7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7],"sequence":1,"command":{"op":"retire","id":1,"extra":true}});
    let mut bytes = Vec::new();
    write_payload(&mut bytes, &value).unwrap();
    assert!(read_request(&mut &bytes[..]).is_err());
}
#[test]
fn structural_rejection_occurs_before_any_output_write() {
    let mut invalids = Vec::new();
    let mut value = record();
    value.process_created = 0;
    invalids.push(value);
    let mut value = record();
    window(&mut value).rect = [i32::MIN, 0, i32::MAX, 32];
    invalids.push(value);
    let mut value = record();
    window(&mut value).normal_position = [0; 4];
    invalids.push(value);
    let mut value = record();
    window(&mut value).extended_style = 0x80000;
    invalids.push(value);
    let mut value = record();
    window(&mut value).layered = Some(Layered {
        color: 0,
        alpha: 1,
        flags: 2,
    });
    invalids.push(value);
    let mut value = record();
    window(&mut value).placement_flags = 8;
    invalids.push(value);
    let mut value = record();
    window(&mut value).show_command = 12;
    invalids.push(value);
    for record in invalids {
        let mut output = Vec::new();
        assert!(write_request(&mut output, &request(Command::Register { id: 1, record })).is_err());
        assert!(output.is_empty());
    }
    for command in [
        Command::Retire { id: 0 },
        Command::Register {
            id: 0,
            record: record(),
        },
    ] {
        assert!(request(command).validate().is_err());
    }
    let mut envelope = request(Command::Shutdown);
    envelope.session = [0; 16];
    assert!(envelope.validate().is_err());
    envelope.session = [7; 16];
    envelope.schema += 1;
    assert!(envelope.validate().is_err());
}
