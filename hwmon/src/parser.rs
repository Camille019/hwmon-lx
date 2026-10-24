// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use pest::Parser;
use pest::iterators::Pair;

use pest_derive::Parser;

use crate::error::Error;

#[derive(Parser)]
#[grammar = "conf.pest"]
pub(crate) struct SensorsConfParser;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Operator {
    Add,
    Sub,
    Multiply,
    Divide,
}

impl Operator {
    pub(crate) fn eval(&self, left: f32, right: f32) -> f32 {
        match self {
            Operator::Add => left + right,
            Operator::Sub => left - right,
            Operator::Multiply => left * right,
            Operator::Divide => left / right,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Function {
    Inv,
    Exp,
    Ln,
}

impl Function {
    pub(crate) fn eval(&self, arg: f32) -> f32 {
        match self {
            Function::Inv => -arg,
            Function::Exp => arg.exp(),
            Function::Ln => arg.ln(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Expr {
    Fn(Function, Box<Expr>),
    Op(Operator, Box<Expr>, Box<Expr>),
    Literal(f32),
    #[default]
    Raw,
}

impl Expr {
    pub(crate) fn eval(&self, raw: f32) -> f32 {
        match self {
            Expr::Fn(inner, expr) => inner.eval(expr.eval(raw)),
            Expr::Op(inner, left, right) => inner.eval(left.eval(raw), right.eval(raw)),
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

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CfgFile {
    buses: Vec<StmtBus>,
    chips: Vec<StmtChip>,
}

impl CfgFile {
    pub(crate) fn buses(&self) -> &Vec<StmtBus> {
        self.buses.as_ref()
    }
    pub(crate) fn chips(&self) -> &Vec<StmtChip> {
        self.chips.as_ref()
    }
    /// Find the configuration statement matching the given chip name.
    ///
    /// Chip names in the configuration file may contain `*` wildcards, as
    /// described in sensors.conf(5). When several statements match, the
    /// last one in the file is returned, as libsensors does.
    pub(crate) fn chip_config(&self, chip_name: &str) -> Option<&StmtChip> {
        self.chips
            .iter()
            .rev()
            .find(|chip| chip.matches_by_name(chip_name))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtBus {
    number: String,
    adapter: String,
}

impl StmtBus {
    pub(crate) fn number(&self) -> &str {
        self.number.as_ref()
    }
    pub(crate) fn adapter(&self) -> &str {
        self.adapter.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtChip {
    names: Vec<String>,
    labels: Vec<StmtLabel>,
    sets: Vec<StmtSet>,
    computes: Vec<StmtCompute>,
    ignores: Vec<StmtIgnore>,
}

impl StmtChip {
    pub(crate) fn names(&self) -> &Vec<String> {
        self.names.as_ref()
    }
    pub(crate) fn labels(&self) -> &Vec<StmtLabel> {
        self.labels.as_ref()
    }
    pub(crate) fn sets(&self) -> &Vec<StmtSet> {
        self.sets.as_ref()
    }
    pub(crate) fn computes(&self) -> &Vec<StmtCompute> {
        self.computes.as_ref()
    }
    pub(crate) fn ignores(&self) -> &Vec<StmtIgnore> {
        self.ignores.as_ref()
    }

    /// Return the label configured for the given feature name, if any.
    ///
    /// When several `label` statements apply to the same name, the first one
    /// in the file is returned.
    pub(crate) fn label_of(&self, feature_name: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|label| label.name() == feature_name)
            .map(StmtLabel::value)
    }

    fn matches_by_name(&self, name: &str) -> bool {
        self.names
            .iter()
            .any(|pattern| chip_name_matches(pattern, name))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtLabel {
    name: String,
    value: String,
}

impl StmtLabel {
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
    pub(crate) fn value(&self) -> &str {
        self.value.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtIgnore {
    name: String,
}

impl StmtIgnore {
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtCompute {
    name: String,
    from_proc: Expr,
    to_proc: Expr,
}

impl StmtCompute {
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtSet {
    name: String,
    value: Expr,
}

impl StmtSet {
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
    pub(crate) fn value(&self) -> &Expr {
        &self.value
    }
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

/// Match a chip name pattern against a chip name.
///
/// `*` matches any sequence of characters, including none, as in
/// sensors.conf(5). All other characters are matched literally.
fn chip_name_matches(pattern: &str, chip_name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let chip_name: Vec<char> = chip_name.chars().collect();

    let mut pattern_index = 0;
    let mut name_index = 0;
    let mut star: Option<usize> = None;
    let mut star_name_index = 0;

    while name_index < chip_name.len() {
        if pattern_index < pattern.len() && pattern[pattern_index] == chip_name[name_index] {
            pattern_index += 1;
            name_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == '*' {
            star = Some(pattern_index);
            star_name_index = name_index;
            pattern_index += 1;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            star_name_index += 1;
            name_index = star_name_index;
        } else {
            return false;
        }
    }

    pattern[pattern_index..].iter().all(|c| *c == '*')
}

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
    compute.name = parse_string(pname);

    let pfrom = pcompute_inner.next().unwrap();
    compute.from_proc = parse_pexpr(pfrom);

    let pto = pcompute_inner.next().unwrap();
    compute.to_proc = parse_pexpr(pto);

    compute
}

fn parse_pignore(pignore: Pair<Rule>) -> StmtIgnore {
    debug_assert!(pignore.as_rule() == Rule::ignore);

    StmtIgnore {
        name: parse_string(pignore),
    }
}

fn parse_plabel(plabel: Pair<Rule>) -> StmtLabel {
    debug_assert!(plabel.as_rule() == Rule::label);

    let mut label = StmtLabel::default();

    for pair in plabel.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                label.name = parse_string(pair);
            }
            Rule::string => {
                label.value = parse_string(pair);
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
                set.name = parse_string(pair);
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

    let mut bus = StmtBus::default();

    for pair in pbus.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                bus.number = parse_string(pair);
            }
            Rule::string => {
                bus.adapter = parse_string(pair);
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

    #[test]
    fn chip_name_matches_wildcards() {
        assert!(chip_name_matches("*", "lm78-isa-0290"));
        assert!(chip_name_matches("lm78-*", "lm78-isa-0290"));
        assert!(chip_name_matches("*-isa-*", "lm78-isa-0290"));
        assert!(chip_name_matches("*-0290", "lm78-isa-0290"));
        assert!(chip_name_matches("lm78-*-0290", "lm78-isa-0290"));
        assert!(chip_name_matches("lm78-isa-0290", "lm78-isa-0290"));

        assert!(!chip_name_matches("lm79-*", "lm78-isa-0290"));
        assert!(!chip_name_matches("*isa", "lm78-isa-0290"));
        assert!(!chip_name_matches("lm78-isa", "lm78-isa-0290"));
        assert!(!chip_name_matches("", "lm78-isa-0290"));
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
        assert_eq!(config.labels[0].value, "ISA voltage");

        let config = conf.chip_config("lm78-i2c-0-2d").unwrap();
        assert_eq!(config.labels[0].value, "Voltage 0");

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
}
