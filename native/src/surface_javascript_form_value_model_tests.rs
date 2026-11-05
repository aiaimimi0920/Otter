//! Rust UTF-16解码/UTF-8计数作为独立语义oracle；模型测试不替代原child的预算门。
use super::document_model_tests::collect;

#[test]
fn javascript_form_value_boundary_fixture_model_mount() {
    let output = collect(include_str!(
        "../tests/fixtures/javascript-form-value-boundaries.js"
    ));
    assert_eq!(output["scene"]["children"][0]["props"]["text"], "ready");
}

#[test]
fn javascript_form_value_model_differential_utf16_scalar_and_newline_validation() {
    let alphabet = [
        0, 9, 10, 13, 31, 127, 128, 2047, 2048, 0xD7FF, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000,
        0xFFFF,
    ];
    let mut units = Vec::new();
    for a in alphabet {
        units.push(vec![a]);
        for b in alphabet {
            units.push(vec![a, b]);
        }
    }
    let mut seed = 0x1365_u32;
    for _ in 0..512 {
        let mut row = Vec::new();
        for _ in 0..5 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            row.push(alphabet[seed as usize % alphabet.len()]);
        }
        units.push(row);
    }
    let rows: Vec<_> = units
        .into_iter()
        .map(|raw| {
            let expected = String::from_utf16(&raw)
                .ok()
                .filter(|text| !text.contains('\0'))
                .map(|text| text.replace(['\r', '\n'], ""))
                .filter(|text| text.len() <= 48 * 1024);
            serde_json::json!([raw, expected])
        })
        .collect();
    let source = format!(
        r#"NeuroSurface.define({{mount({{root}}){{
      root.innerHTML='<p></p><input value="seed">';
      const field=root.children[1], rows={};
      for(const [units,expected] of rows){{
        field.value='preserved';let rejected=false;
        try{{field.value=String.fromCharCode(...units);}}catch{{rejected=true;}}
        if(rejected!==(expected===null)||field.value!==(expected??'preserved'))throw Error('differential value');
      }}
      field.value='seed';root.children[0].textContent='verified:'+rows.length;
    }}}});"#,
        serde_json::to_string(&rows).unwrap()
    );
    let output = collect(&source);
    assert_eq!(
        output["scene"]["children"][0]["props"]["text"],
        format!("verified:{}", rows.len())
    );
}
