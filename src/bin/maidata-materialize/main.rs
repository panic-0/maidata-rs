use serde::Serialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .nth(1)
        .expect("usage: $0 <input> <output> <offset?>");
    let output = std::env::args()
        .nth(2)
        .expect("usage: $0 <input> <output> <offset?>");
    let offset = std::env::args()
        .nth(3)
        .map(|x| x.parse::<f64>().expect("parsing offset failed"))
        .unwrap_or(0.0);

    let content = maidata::app::read_file(input);
    let (insns, state) = maidata::container::parse_maidata_insns(&content);

    let mut mcx = maidata::materialize::MaterializationContext::with_offset(offset);
    let notes = mcx.materialize_insns(insns.iter());

    fn messages_to_value<T: Serialize>(messages: &[maidata::Sp<T>]) -> serde_json::Value {
        messages
            .iter()
            .map(|msg| {
                // serde_json::json!({
                //     "span": msg.span(),
                //     **msg,
                // })
                let json = serde_json::to_value(&**msg).expect("serializing note failed");
                let mut json = json.as_object().expect("json is not an object").clone();
                json.insert(
                    "span".to_string(),
                    serde_json::to_value(msg.span()).expect("serializing span failed"),
                );
                serde_json::Value::Object(json)
            })
            .collect()
    }

    let json = serde_json::json!({
        "chart": notes.iter().map(|note| {
            // serde_json::json!({
            //     "span": note.span(),
            //     **note,
            // })
            let json = serde_json::to_value(&**note).expect("serializing note failed");
            let mut json = json.as_object().expect("json is not an object").clone();
            json.insert("span".to_string(), serde_json::to_value(note.span()).expect("serializing span failed"));
            serde_json::Value::Object(json)
        }).collect::<Vec<_>>(),
        "warnings": messages_to_value(&state.warnings),
        "errors": messages_to_value(&state.errors),
    });
    let json_str = serde_json::to_string_pretty(&json).expect("serializing json failed");
    std::fs::write(output, json_str).expect("writing json file failed");

    Ok(())
}
