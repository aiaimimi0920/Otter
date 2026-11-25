//! Validate message references and external arguments without constraining translation grammar.

use fluent_bundle::FluentResource;
use fluent_syntax::ast::{
    CallArguments, Entry, Expression, InlineExpression, Pattern, PatternElement,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Parameters = BTreeSet<String>;
pub(super) type Contracts = BTreeMap<String, Parameters>;
type Patterns<'a> = BTreeMap<String, &'a Pattern<&'a str>>;

pub(super) fn messages(resources: &[FluentResource]) -> Result<Contracts, String> {
    let mut patterns = Patterns::new();
    let mut messages = BTreeSet::new();
    for resource in resources {
        for entry in resource.entries() {
            match entry {
                Entry::Message(message) => {
                    if let Some(value) = &message.value {
                        insert(&mut patterns, message.id.name.to_owned(), value)?;
                        messages.insert(message.id.name.to_owned());
                    }
                    for attribute in &message.attributes {
                        insert(
                            &mut patterns,
                            format!("{}.{}", message.id.name, attribute.id.name),
                            &attribute.value,
                        )?;
                    }
                }
                Entry::Term(term) => {
                    insert(&mut patterns, format!("-{}", term.id.name), &term.value)?;
                    for attribute in &term.attributes {
                        insert(
                            &mut patterns,
                            format!("-{}.{}", term.id.name, attribute.id.name),
                            &attribute.value,
                        )?;
                    }
                }
                _ => {}
            }
        }
    }
    // Check unreachable branches and private terms as well as public messages.
    let mut contracts = Contracts::new();
    for key in patterns.keys() {
        let parameters = reference(key, &patterns, &mut BTreeSet::new())?;
        if messages.contains(key) {
            contracts.insert(key.clone(), parameters);
        }
    }
    Ok(contracts)
}

fn insert<'a>(
    patterns: &mut Patterns<'a>,
    key: String,
    value: &'a Pattern<&'a str>,
) -> Result<(), String> {
    if patterns.insert(key.clone(), value).is_some() {
        return Err(format!("duplicate interface message or attribute: {key}"));
    }
    Ok(())
}

fn reference(
    key: &str,
    patterns: &Patterns<'_>,
    visiting: &mut BTreeSet<String>,
) -> Result<Parameters, String> {
    let value = patterns
        .get(key)
        .ok_or_else(|| format!("unknown interface message reference: {key}"))?;
    if !visiting.insert(key.to_owned()) {
        return Err(format!("cyclic interface message reference: {key}"));
    }
    let result = pattern(value, patterns, visiting);
    visiting.remove(key);
    result
}

fn pattern(
    value: &Pattern<&str>,
    patterns: &Patterns<'_>,
    visiting: &mut BTreeSet<String>,
) -> Result<Parameters, String> {
    let mut parameters = Parameters::new();
    for element in &value.elements {
        if let PatternElement::Placeable { expression: value } = element {
            parameters.extend(expression(value, patterns, visiting)?);
        }
    }
    Ok(parameters)
}

fn expression(
    value: &Expression<&str>,
    patterns: &Patterns<'_>,
    visiting: &mut BTreeSet<String>,
) -> Result<Parameters, String> {
    match value {
        Expression::Inline(value) => inline(value, patterns, visiting),
        Expression::Select { selector, variants } => {
            let mut parameters = inline(selector, patterns, visiting)?;
            for variant in variants {
                parameters.extend(pattern(&variant.value, patterns, visiting)?);
            }
            Ok(parameters)
        }
    }
}

fn inline(
    value: &InlineExpression<&str>,
    patterns: &Patterns<'_>,
    visiting: &mut BTreeSet<String>,
) -> Result<Parameters, String> {
    match value {
        InlineExpression::VariableReference { id } => Ok([id.name.to_owned()].into()),
        InlineExpression::Placeable { expression: value } => expression(value, patterns, visiting),
        InlineExpression::MessageReference { id, attribute } => {
            let key = reference_key(id.name, attribute.as_ref().map(|value| value.name));
            reference(&key, patterns, visiting)
        }
        InlineExpression::TermReference {
            id,
            attribute,
            arguments: values,
        } => {
            let key = reference_key(
                &format!("-{}", id.name),
                attribute.as_ref().map(|value| value.name),
            );
            reference(&key, patterns, visiting)?;
            // Term variables belong to the term's local argument scope, not the caller.
            match values {
                Some(values) => arguments(values, patterns, visiting),
                None => Ok(Parameters::new()),
            }
        }
        InlineExpression::FunctionReference { id, .. } => Err(format!(
            "unregistered interface formatting function: {}",
            id.name
        )),
        InlineExpression::StringLiteral { .. } | InlineExpression::NumberLiteral { .. } => {
            Ok(Parameters::new())
        }
    }
}

fn arguments(
    values: &CallArguments<&str>,
    patterns: &Patterns<'_>,
    visiting: &mut BTreeSet<String>,
) -> Result<Parameters, String> {
    let mut parameters = Parameters::new();
    for value in &values.positional {
        parameters.extend(inline(value, patterns, visiting)?);
    }
    for value in &values.named {
        parameters.extend(inline(&value.value, patterns, visiting)?);
    }
    Ok(parameters)
}

fn reference_key(id: &str, attribute: Option<&str>) -> String {
    match attribute {
        Some(attribute) => format!("{id}.{attribute}"),
        None => id.to_owned(),
    }
}
