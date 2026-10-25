// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use pest::Parser;
use pest::iterators::Pair;

use pest_derive::Parser;

use crate::config::{
    CfgFile, Expr, Function, Operator, StmtBus, StmtChip, StmtCompute, StmtIgnore, StmtLabel,
    StmtSet,
};
use crate::error::Error;

#[derive(Parser)]
#[grammar = "conf.pest"]
pub(crate) struct SensorsConfParser;

static PRATT_PARSER: LazyLock<pest::pratt_parser::PrattParser<Rule>> = LazyLock::new(|| {
    use pest::pratt_parser::Assoc;
    use pest::pratt_parser::Op;

    pest::pratt_parser::PrattParser::new()
        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
        .op(Op::infix(Rule::mult, Assoc::Left) | Op::infix(Rule::div, Assoc::Left))
        .op(Op::prefix(Rule::inv))
        .op(Op::prefix(Rule::exp))
        .op(Op::prefix(Rule::ln))
});

/// Decode the escape sequences allowed in quoted strings.
///
/// Known C-like escapes are translated to their corresponding character.
/// Any other escaped character is taken literally, so `\?` yields `?`.
fn unescape(string: &str) -> String {
    let mut result = String::with_capacity(string.len());
    let mut chars = string.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }

        match chars.next() {
            Some('a') => result.push('\x07'),
            Some('b') => result.push('\x08'),
            Some('f') => result.push('\x0C'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some('v') => result.push('\x0B'),
            Some(c) => result.push(c),
            None => result.push('\\'),
        }
    }

    result
}

fn parse_string(pair: Pair<Rule>) -> String {
    unescape(pair.into_inner().next().unwrap().as_span().as_str())
}

fn parse_pexpr(pexpr: Pair<Rule>) -> Expr {
    debug_assert!(pexpr.as_rule() == Rule::expr);

    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::raw => Expr::Raw,
            Rule::num => Expr::Literal(primary.as_str().parse::<f64>().unwrap()),
            Rule::expr => parse_pexpr(primary),
            _ => unreachable!(),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::inv => Expr::Fn(Function::Inv, Box::from(rhs)),
            Rule::exp => Expr::Fn(Function::Exp, Box::from(rhs)),
            Rule::ln => Expr::Fn(Function::Ln, Box::from(rhs)),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::add => Expr::Op(Operator::Add, Box::from(lhs), Box::from(rhs)),
            Rule::sub => Expr::Op(Operator::Sub, Box::from(lhs), Box::from(rhs)),
            Rule::mult => Expr::Op(Operator::Multiply, Box::from(lhs), Box::from(rhs)),
            Rule::div => Expr::Op(Operator::Divide, Box::from(lhs), Box::from(rhs)),
            _ => unreachable!(),
        })
        .parse(pexpr.into_inner())
}

fn parse_pcompute(pcompute: Pair<Rule>) -> StmtCompute {
    debug_assert!(pcompute.as_rule() == Rule::compute);

    let statement = pcompute.as_str().to_owned();

    let mut pcompute_inner = pcompute.into_inner();

    let pname = pcompute_inner.next().unwrap();
    let name = parse_string(pname);

    let pfrom = pcompute_inner.next().unwrap();
    let from_proc = parse_pexpr(pfrom);

    let pto = pcompute_inner.next().unwrap();
    let to_proc = parse_pexpr(pto);

    StmtCompute::new(name, statement, from_proc, to_proc)
}

fn parse_pignore(pignore: Pair<Rule>) -> StmtIgnore {
    debug_assert!(pignore.as_rule() == Rule::ignore);

    StmtIgnore::new(parse_string(pignore))
}

fn parse_plabel(plabel: Pair<Rule>) -> StmtLabel {
    debug_assert!(plabel.as_rule() == Rule::label);

    let mut name = String::new();
    let mut value = String::new();

    for pair in plabel.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                name = parse_string(pair);
            }
            Rule::string => {
                value = parse_string(pair);
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    StmtLabel::new(name, value)
}

fn parse_pset(pset: Pair<Rule>) -> StmtSet {
    debug_assert!(pset.as_rule() == Rule::set);

    let mut name = String::new();
    let mut value = Expr::Literal(0.0);

    for pair in pset.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                name = parse_string(pair);
            }
            Rule::expr => {
                value = parse_pexpr(pair);
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    StmtSet::new(name, value)
}

fn parse_pchip(pchip: Pair<Rule>) -> StmtChip {
    debug_assert!(pchip.as_rule() == Rule::chip);

    let mut chip = StmtChip::default();

    for pair in pchip.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                let name = parse_string(pair);
                chip.names.push(name);
            }
            Rule::compute => {
                let compute = parse_pcompute(pair);
                chip.computes.push(compute);
            }
            Rule::ignore => {
                let ignore = parse_pignore(pair);
                chip.ignores.push(ignore);
            }
            Rule::label => {
                let label = parse_plabel(pair);
                chip.labels.push(label);
            }
            Rule::set => {
                let set = parse_pset(pair);
                chip.sets.push(set);
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    chip
}

fn parse_pbus(pbus: Pair<Rule>) -> StmtBus {
    debug_assert!(pbus.as_rule() == Rule::bus);

    let mut number = String::new();
    let mut adapter = String::new();

    for pair in pbus.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                number = parse_string(pair);
            }
            Rule::string => {
                adapter = parse_string(pair);
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    StmtBus::new(number, adapter)
}

fn parse_pfile(pfile: Pair<Rule>) -> CfgFile {
    debug_assert!(pfile.as_rule() == Rule::file);

    let mut buses = Vec::<StmtBus>::new();
    let mut chips = Vec::<StmtChip>::new();

    for pair in pfile.into_inner() {
        match pair.as_rule() {
            Rule::bus => {
                let bus = parse_pbus(pair);
                buses.push(bus)
            }
            Rule::chip => {
                let chip = parse_pchip(pair);
                chips.push(chip)
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    CfgFile::new(buses, chips)
}

pub(crate) fn parse_configuration_str(data: &str) -> Result<CfgFile, Error> {
    let root = SensorsConfParser::parse(Rule::file, data)
        .unwrap()
        .next()
        .unwrap();

    let cfg = parse_pfile(root);

    Ok(cfg)
}

pub(crate) fn parse_configuration_file<P: AsRef<Path>>(path: P) -> Result<CfgFile, Error> {
    let file = fs::read_to_string(path).ok().unwrap();

    parse_configuration_str(&file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_conf_bus_statement() {
        let cfg_str = r#"
bus "i2c-0" "SMBus I801 adapter at 0400"
bus "i2c-1" "SMBus PIIX4 adapter port 2 at 0b00"
"#;
        let expected = CfgFile::new(
            vec![
                StmtBus::new(
                    "i2c-0".to_string(),
                    "SMBus I801 adapter at 0400".to_string(),
                ),
                StmtBus::new(
                    "i2c-1".to_string(),
                    "SMBus PIIX4 adapter port 2 at 0b00".to_string(),
                ),
            ],
            Default::default(),
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_unquoted() {
        let cfg_str = r#"
chip "blah-*"
    label foo bar
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_escaped_newline() {
        let cfg_str = r#"
chip "blah-*"
    label		\
     foo bar
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_immediate_EOF() {
        let cfg_str = r#"
chip "blah-*"
    label foo bar"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_error_0() {
        let cfg_str = r#"
chip "blah-*"
    label ?foo "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("?foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_error_1() {
        let cfg_str = r#"
chip "blah-*"
    label foo% "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo%".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_error_2() {
        let cfg_str = r#"
chip "blah-*"
    label baz$foo "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("baz$foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_error_3() {
        let cfg_str = r#"
chip "blah-*"
    label ! "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("!".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_quoted() {
        let cfg_str = r#"
chip "blah-*"
    label "foo" "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_full_range() {
        let cfg_str = r#"
chip "blah-*"
    label "abcdefg" "hijklmnop"
    label "qrs" "tuv"
    label "wx" "yz"
    label "a0123456789" "982lksdf"
    label "_abcd" "1234_"
    label "_" "foo_bar_baz"
    label "liajesiajef82197fjadf" "blah"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![
                    StmtLabel::new("abcdefg".to_string(), "hijklmnop".to_string()),
                    StmtLabel::new("qrs".to_string(), "tuv".to_string()),
                    StmtLabel::new("wx".to_string(), "yz".to_string()),
                    StmtLabel::new("a0123456789".to_string(), "982lksdf".to_string()),
                    StmtLabel::new("_abcd".to_string(), "1234_".to_string()),
                    StmtLabel::new("_".to_string(), "foo_bar_baz".to_string()),
                    StmtLabel::new("liajesiajef82197fjadf".to_string(), "blah".to_string()),
                ],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_escaped_newline() {
        let cfg_str = r#"
chip "blah-*"
    label		\
     "foo" "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_escaped_chars_like_c() {
        let cfg_str = r#"
chip "blah-*"
    label escapes "\a\b\f\n\r\t\v\\\?\'\""
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new(
                    "escapes".to_string(),
                    "\x07\x08\x0C\n\r\t\x0B\\?\'\"".to_string(),
                )],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_escaped_chars_collapse() {
        let cfg_str = r#"
chip "blah-*"
    label more "\h\e\l\l\o"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("more".to_string(), "hello".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_immediate_EOF() {
        let cfg_str = r#"
chip "blah-*"
    label "foo" "bar""#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_error_no_whitespace() {
        let cfg_str = r#"
chip "blah-*"
    label "foo""bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_quoted_error_no_closing_EOL() {
        let cfg_str = r#"
chip "blah-*"
    label "in0" "foo
    label "in1" "bar"
"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![
                    StmtLabel::new("in0".to_string(), "foo".to_string()),
                    StmtLabel::new("in1".to_string(), "bar".to_string()),
                ],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_quoted_error_no_closing_EOF() {
        let cfg_str = r#"
chip "blah-*"
    label "foo" "bar"#;
        let expected = CfgFile::new(
            Default::default(),
            vec![StmtChip {
                names: vec!["blah-*".to_string()],
                labels: vec![StmtLabel::new("foo".to_string(), "bar".to_string())],
                ..Default::default()
            }],
        );
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_str_compute() {
        let cfg_str = r#"
chip "lm78-*"

    compute in1 @*(1+120/56) - 4.096*120/56, -(@ + 4.096*120/56)/(1+120/56)
    compute in2 @*(1+120/56) - 4.096*120/56, `(@ + 4.096*120/56)/(1+120/56)
    compute in3 @*(1+120/56) - 4.096*120/56, ^(@ + 4.096*120/56)/(1+120/56)
"#;
        assert_eq!(parse_configuration_str(cfg_str).is_ok(), true);
    }

    #[test]
    fn chip_config_finds_last_matching_statement() {
        let cfg_str = r#"
chip "lm78-*"
    label in0 "Voltage 0"
chip "*-isa-*"
    label in0 "ISA voltage"
chip "nothing-*"
    label in0 "Nothing"
"#;
        let conf = parse_configuration_str(cfg_str).unwrap();

        let config = conf.chip_config("lm78-isa-0290").unwrap();
        assert_eq!(config.labels[0].value(), "ISA voltage");

        let config = conf.chip_config("lm78-i2c-0-2d").unwrap();
        assert_eq!(config.labels[0].value(), "Voltage 0");

        assert!(conf.chip_config("nothing-isa-0000").is_some());
        assert!(conf.chip_config("other-i2c-0-2d").is_none());
        assert!(conf.chip_config("otherchip-i2c-0-2d").is_none());
    }

    #[test]
    fn chip_label_returns_first_match() {
        let cfg_str = r#"
chip "lm78-*"
    label in0 "Voltage 0"
    label in0 "Main voltage"
    label fan1 "Fan"
"#;
        let conf = parse_configuration_str(cfg_str).unwrap();
        let config = conf.chip_config("lm78-isa-0290").unwrap();

        assert_eq!(config.label_of("in0"), Some("Voltage 0"));
        assert_eq!(config.label_of("fan1"), Some("Fan"));
        assert_eq!(config.label_of("temp1"), None);
    }

    #[test]
    fn compute_statement_evaluates_both_ways() {
        let cfg_str = r#"
chip "lm78-*"
    compute in1 @ * 2 + 1, (@ - 1) / 2
"#;
        let conf = parse_configuration_str(cfg_str).unwrap();
        let config = conf.chip_config("lm78-isa-0290").unwrap();
        let compute = config.compute_of("in1").unwrap();

        assert_eq!(compute.eval_from(1.0), 3.0);
        assert_eq!(compute.eval_to(3.0), 1.0);
        assert_eq!(compute.statement(), "compute in1 @ * 2 + 1, (@ - 1) / 2");
        assert!(config.compute_of("in2").is_none());
    }
}
