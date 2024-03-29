#![allow(dead_code)]
#![feature(test)]
#![feature(collections)]

extern crate rand;
extern crate test;
extern crate time;

use std::rc::Rc;
use rand::distributions::{IndependentSample, Range};

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Hash, Debug)]
enum Value {
    Least,
    String(Rc<String>),
    Greatest,
}

// TODO make this a macro
fn lit_value(string: &str) -> Value {
    Value::String(Rc::new(string.to_string()))
}

type Row = Vec<Value>;

// TODO make this a macro
fn lit_row(row: &[&str]) -> Row {
    row.iter().map(|value| lit_value(value)).collect()
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Hash, Debug)]
struct Table {
    num_columns: usize,
    rows: Vec<Row>,
}

impl Table {
    fn from_rows(num_columns: usize, mut rows: Vec<Row>) -> Table {
        rows.sort();
        rows.push(vec![Value::Greatest; num_columns]);
        Table {
            num_columns: num_columns,
            rows: rows,
        }
    }
}

// TODO make this a macro
fn lit_table(rows: &[&[&str]]) -> Table {
    assert!(rows.len() > 0);
    let num_columns = rows[0].len();
    assert!(rows.iter().all(|row| row.len() == num_columns));
    Table::from_rows(num_columns, rows.iter().map(|row| lit_row(row)).collect())
}

impl Table {
    fn next<'a>(&self, row: &Row, inclusive: bool, hint: &mut usize) -> &Row {
        match (self.rows[*hint] == *row, inclusive) {
            (true, true) => *hint = *hint,
            (true, false) => *hint = *hint+1,
            (_, _) => match (self.rows.binary_search(row), inclusive) {
                (Ok(i), true) => *hint = i,
                (Ok(i), false) => *hint = i+1,
                (Err(i), _) => *hint = i,
            }
        }
        &self.rows[*hint]
    }
}

#[test]
fn test_next() {
    let table = lit_table(&[
        &["a", "a", "a"],
        &["a", "b", "a"],
        &["a", "a", "b"],
        ]);
    assert_eq!(table.next(&lit_row(&["a", "a", "a"]), true, &mut 0), &lit_row(&["a", "a", "a"]));
    assert_eq!(table.next(&lit_row(&["a", "a", "a"]), false, &mut 0), &lit_row(&["a", "a", "b"]));
    assert_eq!(table.next(&lit_row(&["a", "a", "b"]), true, &mut 0), &lit_row(&["a", "a", "b"]));
    assert_eq!(table.next(&lit_row(&["a", "a", "b"]), false, &mut 0), &lit_row(&["a", "b", "a"]));
    assert_eq!(table.next(&lit_row(&["a", "a", "c"]), true, &mut 0), &lit_row(&["a", "b", "a"]));
    assert_eq!(table.next(&lit_row(&["a", "a", "c"]), false, &mut 0), &lit_row(&["a", "b", "a"]));
    assert_eq!(table.next(&lit_row(&["a", "b", "a"]), true, &mut 0), &lit_row(&["a", "b", "a"]));
    assert_eq!(table.next(&lit_row(&["a", "b", "a"]), false, &mut 0), &vec![Value::Greatest; 3]);
    assert_eq!(table.next(&lit_row(&["a", "c", "a"]), true, &mut 0), &vec![Value::Greatest; 3]);
    assert_eq!(table.next(&lit_row(&["a", "c", "a"]), false, &mut 0), &vec![Value::Greatest; 3]);
}

struct RowClause {
    mapping: Vec<usize>,
    table: Table,
}

struct RowClauseState {
    hint: usize,
    internal_buffer: Row,
    external_buffer: Row,
}

impl RowClause {
    fn new (mapping: Vec<usize>, table: Table) -> RowClause {
        RowClause {
            mapping: mapping,
            table: table,
        }
    }
}

impl RowClauseState {
    fn new (clause: &RowClause, num_variables: usize) -> RowClauseState {
        RowClauseState {
            hint: 0,
            internal_buffer: vec![Value::Least; clause.table.num_columns],
            external_buffer: vec![Value::Least; num_variables],
        }
    }
}

impl RowClause {
    fn next<'a>(&'a self, state: &'a mut RowClauseState, row: &Row, inclusive: bool) -> &Row {
        for (internal_column, external_column) in self.mapping.iter().enumerate() {
            state.internal_buffer[internal_column] = row[*external_column].clone();
        }
        let &mut RowClauseState { ref mut hint, .. } = state;
        let next_row = self.table.next(&state.internal_buffer, inclusive, hint);
        let mut changed = false;
        for (external_column, external_value) in row.iter().enumerate() {
            match self.mapping.iter().position(|c| *c == external_column) {
                None =>
                    state.external_buffer[external_column] = if changed { Value::Least } else { external_value.clone() },
                Some(internal_column) => {
                    let internal_value = &next_row[internal_column];
                    changed = changed || (internal_value != external_value);
                    state.external_buffer[external_column] = internal_value.clone();
                },
            }
        }
        &state.external_buffer
    }
}

struct Query {
    num_variables: usize,
    clauses: Vec<RowClause>,
}

impl Query {
    #[inline(never)]
    fn run(&self) -> Vec<Row> {
        let mut variables = vec![Value::Least; self.num_variables];
        let mut states: Vec<RowClauseState> = self.clauses.iter().map(|clause| RowClauseState::new(clause, self.num_variables)).collect();
        let mut results = vec![];
        while variables[0] != Value::Greatest {
            let changed;
            {
                let next_variables = states.iter_mut().zip(self.clauses.iter()).map(|(state, clause)| clause.next(state, &variables, true)).max().unwrap();
                changed = *next_variables != variables;
                variables[..].clone_from_slice(&next_variables[..]);
            }
            if !changed {
                results.push(variables.clone());
                let next_variables = states.iter_mut().zip(self.clauses.iter()).map(|(state, clause)| clause.next(state, &variables, false)).min().unwrap();
                variables[..].clone_from_slice(&next_variables[..]);
            }
        }
        results
    }
}

#[test]
fn test_banned_users() {
    let users = lit_table(&[
        &["0", "a@a"],
        &["2", "c@c"],
        &["3", "b@b"],
        &["4", "b@b"],
        ]);

    let logins = lit_table(&[
        &["2", "0.0.0.0"],
        &["2", "1.1.1.1"],
        &["4", "1.1.1.1"],
        ]);

    let bans = lit_table(&[
        &["1.1.1.1"],
        &["2.2.2.2"],
        ]);

    let results = join(3, vec![
            RowClause::new(vec![0,2], users),
            RowClause::new(vec![0,1], logins),
            RowClause::new(vec![1], bans),
        ]);

    assert_eq!(results, vec![lit_row(&["2", "1.1.1.1", "c@c"]), lit_row(&["4", "1.1.1.1", "b@b"])]);
}

#[test]
fn test_paths0() {
    let edges = lit_table(&[
        &["a", "b"],
        &["b", "c"],
        &["c", "d"],
        &["d", "b"],
        ]);

    let edges_rev = lit_table(&[
        &["b", "a"],
        &["c", "b"],
        &["d", "c"],
        &["b", "d"],
        ]);

    let results = join(3, vec![
            RowClause::new(vec![0,1], edges),
            RowClause::new(vec![1,2], edges_rev)
        ]);

    assert_eq!(results, vec![
            lit_row(&["a", "b", "a"]),
            lit_row(&["a", "b", "d"]),
            lit_row(&["b", "c", "b"]),
            lit_row(&["c", "d", "c"]),
            lit_row(&["d", "b", "a"]),
            lit_row(&["d", "b", "d"]),
        ]);
}

#[test]
fn test_paths1() {
    let edges = lit_table(&[
        &["a", "b"],
        &["b", "c"],
        &["c", "d"],
        &["d", "b"],
        ]);

    let edges_rev = lit_table(&[
        &["b", "a"],
        &["c", "b"],
        &["d", "c"],
        &["b", "d"],
        ]);

    let results = join(3, vec![
            RowClause::new(vec![1,2], edges),
            RowClause::new(vec![0,1], edges_rev)
    ]);

    assert_eq!(results, vec![
        lit_row(&["b", "a", "b"]),
        lit_row(&["b", "d", "b"]),
        lit_row(&["c", "b", "c"]),
        lit_row(&["d", "c", "d"]),
    ]);
}

fn bench_join(bench_size: usize) {
    let between = Range::new(0, bench_size);
    let mut rng = rand::thread_rng();

    let mut users = vec![];
    let mut logins = vec![];
    let mut bans = vec![];

    for i in (0..bench_size) {
        users.push(lit_row(&[&format!("user{}", i), &format!("email{}", i)]));
    }

    for i in (0..bench_size) {
        let user = between.ind_sample(&mut rng);
        logins.push(lit_row(&[&format!("user{}", user), &format!("ip{}", i)]));
    }

    for i in (0..bench_size) {
        bans.push(lit_row(&[&format!("ip{}", i)]));
    }

    let start = time::precise_time_s();
    let users = Table::from_rows(2, users);
    let logins = Table::from_rows(2, logins);
    let bans = Table::from_rows(1, bans);
    let end = time::precise_time_s();
    println!("index: {}s", end - start);

    let query = Query{
        num_variables: 3,
        clauses: vec![
            RowClause::new(vec![0,2], users),
            RowClause::new(vec![0,1], logins),
            RowClause::new(vec![1], bans),
        ],
    };

    let start = time::precise_time_s();
    let results = query.run();
    let end = time::precise_time_s();
    println!("solve: {}s", end - start);

    let start = time::precise_time_s();
    drop(query);
    let end = time::precise_time_s();
    println!("erase: {}s", end - start);

    println!("{:?} results", results.len());
}

fn main() {
    bench_join(1_000_000);
}