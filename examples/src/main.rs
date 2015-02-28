#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Hash, Debug)]
enum Value {
	Least,
	String(String),
	Greatest
}

// TODO make this a macro
fn lit_value(string: &str) -> Value {
	Value::String(string.to_string())
}

type Row = Vec<Value>;

// TODO make this a macro
fn lit_row(row: &[&str]) -> Row {
	row.iter().map(|value| lit_value(value)).collect()
}

struct Table {
	rows: Vec<Row>
}

impl Table {
	fn from_rows(num_columns: usize, mut rows: Vec<Row>) -> Table {
		rows.sort();
		rows.push(vec![Value::Greatest; num_columns]);
		Table {
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

trait Skippable {
	fn next(&self, row: &Row, inclusive: bool) -> &Row;
}

impl Skippable for Table {
 	fn next(&self, row: &Row, inclusive: bool) -> &Row {
 		let rows = &self.rows;
 		let mut lo = 0; // lo <= target
 		let mut hi = row.len(); // target < hi
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

fn main() {
}