// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


use std::fs;
use std::path::Path;

use lazy_static::lazy_static;

use pest::Parser;
use pest::iterators::Pair;
use pest::prec_climber;
use pest_derive::Parser;

use crate::error::Error;

#[derive(Parser)]
#[grammar = "conf.pest"]
pub(crate) struct SensorsConfParser;

#[derive(Debug)]
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

#[derive(Debug)]
enum Function {
    Inv,
    Exp,
    Ln,
}

impl Function {
    fn eval(&self, arg: f32) -> f32 {
        match self {
            Function::Inv => arg * -1.0,
            Function::Exp => arg.exp(),
            Function::Ln => arg.ln(),
        }
    }
}

#[derive(Debug)]
enum Expr {
    Fn(Function, Box<Expr>),
    Op(Operator, Box<Expr>, Box<Expr>),
    Literal(f32),
    Raw,
}

impl Default for Expr {
    fn default() -> Self { Expr::Raw }
}

impl Expr {
    fn eval(&self, raw: f32) -> f32 {
        match self {
            Expr::Fn(ref inner, ref expr ) => inner.eval(expr.eval(raw)),
            Expr::Op(ref inner, ref left, ref right) => inner.eval(left.eval(raw), right.eval(raw)),
            Expr::Literal( inner) => *inner,
            Expr::Raw => raw,
        }
    }
}


//struct ChipName {
//    prefix: String,
//    bus: Bus,
//    address: u32,
//}

#[derive(Debug, Default)]
pub(crate) struct CfgFile {
//    bus: Vec<>,
    chips: Vec<StmtChip>,
}

#[derive(Debug, Default)]
struct StmtChip {
    names: Vec<String>,
    labels: Vec<StmtLabel>,
    sets: Vec<StmtSet>,
    computes: Vec<StmtCompute>,
    ignores: Vec<StmtIgnore>,
}

#[derive(Debug, Default)]
struct StmtLabel {
    name: String,
    value: String,
}

#[derive(Debug, Default)]
struct StmtIgnore {
    name: String
}

#[derive(Debug, Default)]
struct StmtCompute {
    name: String,
    from_proc: Expr,
    to_proc: Expr,
}

#[derive(Debug, Default)]
struct StmtSet {
    name: String,
    value: Expr, // TODO
}


lazy_static! {
    static ref PREC_CLIMBER: prec_climber::PrecClimber<Rule> = {
        use Rule::*;
        use prec_climber::Assoc::*;

        prec_climber::PrecClimber::new(vec![
            prec_climber::Operator::new(add, Left) | prec_climber::Operator::new(sub, Left),
            prec_climber::Operator::new(mult, Left) | prec_climber::Operator::new(div, Left),
        ])
    };
}

fn parse_pexpr(pexpr: Pair<Rule>) -> Expr {
    debug_assert!(pexpr.as_rule() == Rule::expr);

    PREC_CLIMBER.climb(
        pexpr.into_inner(),
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::raw => Expr::Raw,
            Rule::num => Expr::Literal(pair.as_str().parse::<f32>().unwrap()),
            Rule::function => unimplemented!(),
            Rule::expr => parse_pexpr(pair),
            _ => unreachable!(),
        },
        |lhs: Expr, op: Pair<Rule>, rhs: Expr| match op.as_rule() {
            Rule::add   => Expr::Op(Operator::Add, Box::from(lhs), Box::from(rhs)),
            Rule::sub   => Expr::Op(Operator::Sub, Box::from(lhs), Box::from(rhs)),
            Rule::mult  => Expr::Op(Operator::Multiply, Box::from(lhs), Box::from(rhs)),
            Rule::div   => Expr::Op(Operator::Divide, Box::from(lhs), Box::from(rhs)),
            _ => unreachable!(),
        },
    )
}


fn parse_pcompute(pcompute: Pair<Rule>) -> StmtCompute {
    debug_assert!(pcompute.as_rule() == Rule::compute);

    let mut compute = StmtCompute::default();

    let mut pcompute_inner = pcompute.into_inner();

    let pname = pcompute_inner.next().unwrap();
    compute.name = pname.into_inner().next().unwrap().as_span().as_str().to_string();

    let pfrom = pcompute_inner.next().unwrap();
    compute.from_proc = parse_pexpr(pfrom);

    let pto = pcompute_inner.next().unwrap();
    compute.to_proc = parse_pexpr(pto);

    compute
}

fn parse_pignore(pignore: Pair<Rule>) -> StmtIgnore {
    debug_assert!(pignore.as_rule() == Rule::ignore);

    let ignore = StmtIgnore { name: pignore.into_inner().next().unwrap().as_span().as_str().to_string() };

    ignore
}

fn parse_plabel(plabel: Pair<Rule>) -> StmtLabel {
    debug_assert!(plabel.as_rule() == Rule::label);

    let mut label = StmtLabel::default();

    for pair in plabel.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                label.name = pair.into_inner().next().unwrap().as_span().as_str().to_string();
            },
            Rule::string => {
                label.value = pair.into_inner().next().unwrap().as_span().as_str().to_string();
            },
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            },
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
                set.name = pair.into_inner().next().unwrap().as_span().as_str().to_string();
            },
            Rule::expr => {
                set.value = parse_pexpr(pair);
            },
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            },
        }

    }

    set
}

fn parse_pchip(pchip: Pair<Rule>) -> StmtChip {
    debug_assert!(pchip.as_rule() == Rule::chip);

    let mut chip  = StmtChip::default();

    for pair in pchip.into_inner() {
        match pair.as_rule() {
            Rule::name => {
                let name = pair.into_inner().next().unwrap().as_span().as_str();
                chip.names.push(String::from(name));
            },
            Rule::compute => {
                let compute = parse_pcompute(pair);
                chip.computes.push(compute);
            },
            Rule::ignore => {
                let ignore = parse_pignore(pair);
                chip.ignores.push(ignore);
            },
            Rule::label => {
                let label = parse_plabel(pair);
                chip.labels.push(label);
            },
            Rule::set => {
                let set = parse_pset(pair);
                chip.sets.push(set);
            },
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            },
        }

    }

    chip
}

fn parse_pfile(pfile: Pair<Rule>) -> CfgFile {
    debug_assert!(pfile.as_rule() == Rule::file);

    let mut cfg= CfgFile::default();

    for pair in pfile.into_inner() {
        match pair.as_rule() {
            Rule::bus => {},
            Rule::chip => {
                let chip = parse_pchip(pair);
                cfg.chips.push(chip)
            },
            _ => {
                log::debug!("Found bad pair: {:#?}", pair);
                unreachable!()
            },
        }
    }

    cfg
}

pub(crate) fn parse_configuration_str(data: &str) -> Result<CfgFile, Error> {
    let root = SensorsConfParser::parse(Rule::file, data).unwrap().next().unwrap();

    let cfg = parse_pfile(root);

    Ok(cfg)
}

pub(crate) fn parse_configuration_file<P: AsRef<Path>>(path: P) -> Result<CfgFile, Error> {
    let file = fs::read_to_string(path).ok().unwrap();

    parse_configuration_str(&file)
}
