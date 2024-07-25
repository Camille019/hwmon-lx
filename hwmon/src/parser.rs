// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::error::Error;

#[derive(Parser)]
#[grammar = "conf.pest"]
pub(crate) struct SensorsConfParser;

#[derive(Debug, PartialEq)]
enum Operator {
    Add,
    Sub,
    Multiply,
    Divide,
}

impl Operator {
    fn eval(&self, left: f32, right: f32) -> f32 {
        match self {
            Operator::Add => left + right,
            Operator::Sub => left - right,
            Operator::Multiply => left * right,
            Operator::Divide => left / right,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Function {
    Inv,
    Exp,
    Ln,
}

impl Function {
    fn eval(&self, arg: f32) -> f32 {
        match self {
            Function::Inv => -arg,
            Function::Exp => arg.exp(),
            Function::Ln => arg.ln(),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
enum Expr {
    Fn(Function, Box<Expr>),
    Op(Operator, Box<Expr>, Box<Expr>),
    Literal(f32),
    #[default]
    Raw,
}

impl Expr {
    fn eval(&self, raw: f32) -> f32 {
        match self {
            Expr::Fn(ref inner, ref expr) => inner.eval(expr.eval(raw)),
            Expr::Op(ref inner, ref left, ref right) => inner.eval(left.eval(raw), right.eval(raw)),
            Expr::Literal(inner) => *inner,
            Expr::Raw => raw,
        }
    }
}

//struct ChipName {
//    prefix: String,
//    bus: Bus,
//    address: u32,
//}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct CfgFile {
    buses: Vec<StmtBus>,
    chips: Vec<StmtChip>,
}

#[derive(Debug, Default, PartialEq)]
struct StmtBus {
    number: String,
    adapter: String,
}

#[derive(Debug, Default, PartialEq)]
struct StmtChip {
    names: Vec<String>,
    labels: Vec<StmtLabel>,
    sets: Vec<StmtSet>,
    computes: Vec<StmtCompute>,
    ignores: Vec<StmtIgnore>,
}

#[derive(Debug, Default, PartialEq)]
struct StmtLabel {
    name: String,
    value: String,
}

#[derive(Debug, Default, PartialEq)]
struct StmtIgnore {
    name: String,
}

#[derive(Debug, Default, PartialEq)]
struct StmtCompute {
    name: String,
    from_proc: Expr,
    to_proc: Expr,
}

#[derive(Debug, Default, PartialEq)]
struct StmtSet {
    name: String,
    value: Expr,
}

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

fn parse_pexpr(pexpr: Pair<Rule>) -> Expr {
    debug_assert!(pexpr.as_rule() == Rule::expr);

    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::raw => Expr::Raw,
            Rule::num => Expr::Literal(primary.as_str().parse::<f32>().unwrap()),
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

    let mut compute = StmtCompute::default();

    let mut pcompute_inner = pcompute.into_inner();

    let pname = pcompute_inner.next().unwrap();
    compute.name = pname
        .into_inner()
        .next()
        .unwrap()
        .as_span()
        .as_str()
        .to_string();

    let pfrom = pcompute_inner.next().unwrap();
    compute.from_proc = parse_pexpr(pfrom);

    let pto = pcompute_inner.next().unwrap();
    compute.to_proc = parse_pexpr(pto);

    compute
}

fn parse_pignore(pignore: Pair<Rule>) -> StmtIgnore {
    debug_assert!(pignore.as_rule() == Rule::ignore);

    let ignore = StmtIgnore {
        name: pignore
            .into_inner()
            .next()
            .unwrap()
            .as_span()
            .as_str()
            .to_string(),
    };

    ignore
}

fn parse_plabel(plabel: Pair<Rule>) -> StmtLabel {
    debug_assert!(plabel.as_rule() == Rule::label);

    let mut label = StmtLabel::default();

    for pair in plabel.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                label.name = pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_string();
            }
            Rule::string => {
                label.value = pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_string();
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    label
}

fn parse_pset(pset: Pair<Rule>) -> StmtSet {
    debug_assert!(pset.as_rule() == Rule::set);

    let mut set = StmtSet::default();

    for pair in pset.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                set.name = pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_string();
            }
            Rule::expr => {
                set.value = parse_pexpr(pair);
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    set
}

fn parse_pchip(pchip: Pair<Rule>) -> StmtChip {
    debug_assert!(pchip.as_rule() == Rule::chip);

    let mut chip = StmtChip::default();

    for pair in pchip.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                let name = pair.into_inner().next().unwrap().as_span().as_str();
                chip.names.push(String::from(name));
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

    let mut bus = StmtBus::default();

    for pair in pbus.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                bus.number = pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_string();
            }
            Rule::string => {
                bus.adapter = pair
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_string();
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    bus
}

fn parse_pfile(pfile: Pair<Rule>) -> CfgFile {
    debug_assert!(pfile.as_rule() == Rule::file);

    let mut cfg = CfgFile::default();

    for pair in pfile.into_inner() {
        match pair.as_rule() {
            Rule::bus => {
                let bus = parse_pbus(pair);
                cfg.buses.push(bus)
            }
            Rule::chip => {
                let chip = parse_pchip(pair);
                cfg.chips.push(chip)
            }
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            }
        }
    }

    cfg
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
        let expected = CfgFile {
            buses: vec![
                StmtBus {
                    number: String::from("i2c-0"),
                    adapter: String::from("SMBus I801 adapter at 0400"),
                },
                StmtBus {
                    number: String::from("i2c-1"),
                    adapter: String::from("SMBus PIIX4 adapter port 2 at 0b00"),
                },
            ],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_unquoted() {
        let cfg_str = r#"
chip "blah-*"
    label foo bar
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
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
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_immediate_EOF() {
        let cfg_str = r#"
chip "blah-*"
    label foo bar"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_error_0() {
        let cfg_str = r#"
chip "blah-*"
    label ?foo "bar"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("?foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_error_1() {
        let cfg_str = r#"
chip "blah-*"
    label foo% "bar"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo%"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_error_2() {
        let cfg_str = r#"
chip "blah-*"
    label baz$foo "bar"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("baz$foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_error_3() {
        let cfg_str = r#"
chip "blah-*"
    label ! "bar"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("!"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_quoted() {
        let cfg_str = r#"
chip "blah-*"
    label "foo" "bar"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
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
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![
                    StmtLabel {
                        name: String::from("abcdefg"),
                        value: String::from("hijklmnop"),
                    },
                    StmtLabel {
                        name: String::from("qrs"),
                        value: String::from("tuv"),
                    },
                    StmtLabel {
                        name: String::from("wx"),
                        value: String::from("yz"),
                    },
                    StmtLabel {
                        name: String::from("a0123456789"),
                        value: String::from("982lksdf"),
                    },
                    StmtLabel {
                        name: String::from("_abcd"),
                        value: String::from("1234_"),
                    },
                    StmtLabel {
                        name: String::from("_"),
                        value: String::from("foo_bar_baz"),
                    },
                    StmtLabel {
                        name: String::from("liajesiajef82197fjadf"),
                        value: String::from("blah"),
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };
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
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_escaped_chars_like_c() {
        let cfg_str = r#"
chip "blah-*"
    label escapes "\a\b\f\n\r\t\v\\\?\'\""
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("escapes"),
                    value: String::from("\x07\x08\x0C\n\r\t\x0B\\?\'\""),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_escaped_chars_collapse() {
        let cfg_str = r#"
chip "blah-*"
    label more "\h\e\l\l\o"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("more"),
                    value: String::from("hello"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_immediate_EOF() {
        let cfg_str = r#"
chip "blah-*"
    label "foo" "bar""#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, true);
    }

    #[test]
    fn parse_conf_name_quoted_error_no_whitespace() {
        let cfg_str = r#"
chip "blah-*"
    label "foo""bar"
"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
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
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![
                    StmtLabel {
                        name: String::from("in0"),
                        value: String::from("foo"),
                    },
                    StmtLabel {
                        name: String::from("in1"),
                        value: String::from("bar"),
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };
        let conf = parse_configuration_str(cfg_str).unwrap_or_default();
        assert_eq!(conf == expected, false);
    }

    #[test]
    fn parse_conf_name_quoted_error_no_closing_EOF() {
        let cfg_str = r#"
chip "blah-*"
    label "foo" "bar"#;
        let expected = CfgFile {
            chips: vec![StmtChip {
                names: vec![String::from("blah-*")],
                labels: vec![StmtLabel {
                    name: String::from("foo"),
                    value: String::from("bar"),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
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
}
