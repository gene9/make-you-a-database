#![allow(dead_code)]
#![feature(test)]

extern crate rand;
extern crate test;
extern crate time;

use std::rc::Rc;
use rand::distributions::{IndependentSample, Range};

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Hash, Debug)]
enum Value {
	Least,
	String(Rc<String>),
	Greatest
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
	rows: Vec<Row>
}

impl Table {
	fn from_rows(num_columns: usize, mut rows: Vec<Row>) -> Table {
		rows.sort();
		rows.push(vec![Value::Greatest; num_columns]);
		Table {
			num_columns: num_columns,
			rows: rows
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
 	fn next(&self, row: &Row, inclusive: bool) -> &Row {
 		let rows = &self.rows;
 		let mut lo = 0; // lo <= target
 		let mut hi = rows.len(); // target < hi
 		loop {
 			if lo >= hi { break; }
 			let mid = lo + ((hi - lo) / 2);
 			if *row <= rows[mid] {
 				hi = mid;
 			} else {
 				lo = mid + 1;
 			}
 		}
 		if (*row == rows[lo]) && !inclusive {
 			lo += 1;
 		}
 		&rows[lo]
 	}
}

#[test]
fn test_next() {
	let table = lit_table(&[
		&["a", "a", "a"],
		&["a", "b", "a"],
		&["a", "a", "b"]
		]);
 	assert_eq!(table.next(&lit_row(&["a", "a", "a"]), true), &lit_row(&["a", "a", "a"]));
 	assert_eq!(table.next(&lit_row(&["a", "a", "a"]), false), &lit_row(&["a", "a", "b"]));
 	assert_eq!(table.next(&lit_row(&["a", "a", "b"]), true), &lit_row(&["a", "a", "b"]));
 	assert_eq!(table.next(&lit_row(&["a", "a", "b"]), false), &lit_row(&["a", "b", "a"]));
 	assert_eq!(table.next(&lit_row(&["a", "a", "c"]), true), &lit_row(&["a", "b", "a"]));
 	assert_eq!(table.next(&lit_row(&["a", "a", "c"]), false), &lit_row(&["a", "b", "a"]));
 	assert_eq!(table.next(&lit_row(&["a", "b", "a"]), true), &lit_row(&["a", "b", "a"]));
 	assert_eq!(table.next(&lit_row(&["a", "b", "a"]), false), &vec![Value::Greatest; 3]);
 	assert_eq!(table.next(&lit_row(&["a", "c", "a"]), true), &vec![Value::Greatest; 3]);
 	assert_eq!(table.next(&lit_row(&["a", "c", "a"]), false), &vec![Value::Greatest; 3]);
}

struct RowClause {
	mapping: Vec<usize>,
	table: Table
}

impl RowClause {
	fn new (mapping: Vec<usize>, table: Table) -> RowClause {
	    return RowClause {
		    mapping: mapping,
			table: table
		}
	}
}

impl RowClause {
	fn next(&self, row: &Row, inclusive: bool) -> Row {
		// TODO do the vec allocations in here matter?
		let internal = self.mapping.iter().map(|external_ix| row[*external_ix].clone()).collect();
		let next = self.table.next(&internal, inclusive);
		let mut external = row.clone();
		for (next_value, (prev_value, external_ix)) in next.iter().zip(internal.iter().zip(self.mapping.iter())) {
			if next_value != prev_value {
			    external[*external_ix] = next_value.clone();
			    for external_cell in external[(external_ix + 1)..].iter_mut() {
			    	*external_cell = Value::Least;
			    }
				break;
			}
		}
		external
	}
}

fn join(num_variables: usize, clauses: Vec<RowClause>) -> Vec<Row> {
	let mut variables = vec![Value::Least; num_variables];
	let mut results = vec![];
	loop {
			let mut next_variables = clauses.iter().map(|clause| clause.next(&variables, true)).max().unwrap();
			if next_variables[0] == Value::Greatest {
				break;
			}
			if next_variables == variables {
				results.push(variables.clone());
				next_variables = clauses.iter().map(|clause| clause.next(&variables, false)).min().unwrap();
				if next_variables[0] == Value::Greatest {
					break;
				}
			}
			variables = next_variables;
		}
	results
}

#[test]
fn test_join() {
	let users = lit_table(&[
		&["0", "a@a"],
		&["2", "c@c"],
		&["3", "b@b"],
		&["4", "b@b"]
		]);

	let logins = lit_table(&[
		&["2", "0.0.0.0"],
		&["2", "1.1.1.1"],
		&["4", "1.1.1.1"]
		]);

	let bans = lit_table(&[
		&["1.1.1.1"],
		&["2.2.2.2"]
		]);

	let results = join(3, vec![
			RowClause::new(vec![0,2], users),
			RowClause::new(vec![0,1], logins),
			RowClause::new(vec![1], bans)
		]);

	assert_eq!(results, vec![lit_row(&["2", "1.1.1.1", "c@c"]), lit_row(&["4", "1.1.1.1", "b@b"])]);
}

fn bench_join(bench_size: usize) {
    let between = Range::new(0, bench_size);
    let mut rng = rand::thread_rng();

	let mut users = vec![];
	let mut logins = vec![];
	let mut bans = vec![];

	for i in (0..bench_size) {
		users.push(lit_row(&[&format!("user{}", i), &format!("email{}", i)]))
	}

	for i in (0..bench_size) {
		let user = between.ind_sample(&mut rng);
		logins.push(lit_row(&[&format!("user{}", user), &format!("ip{}", i)]));
	}

	for i in (0..bench_size) {
		bans.push(lit_row(&[&format!("ip{}", i)]));
	}

	let start = time::precise_time_s();
	test::black_box(join(3, vec![
		RowClause::new(vec![0,2], Table::from_rows(2, users)),
		RowClause::new(vec![0,1], Table::from_rows(2, logins)),
		RowClause::new(vec![1], Table::from_rows(1, bans))
	]));
	let end = time::precise_time_s();
	println!("{}s", end - start);
}

fn main() {
	bench_join(1_000);
	// bench_join(1_000);
	// bench_join(1_000);
	// bench_join(10_000);
	// bench_join(10_000);
	// bench_join(10_000);
	// bench_join(100_000);
}