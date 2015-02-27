TODO

* Change unmap/remap to sensible names
* Add a proof for the join algorithm
* Make a reference implementation
* Query language section
* Talk about relationship between variable ordering, selectivity, skew etc
* Feature section

# Make you a database

Relational databases are often seen as complex, mysterious things. Most of that complexity exists for historical reasons. We are going to build a simple, reasonably fast relational database as a library - a collection of pieces that you can use together or individually.

## Plan

We will start as simple as possible and add features incrementally. A rough roadmap for the core looks like this:

* Tables as arrays of tuples
* Write a join algorithm which can join on a single, integer variable
* Handle multiple joins
* Add a simple query language

From there we can add more features in any order:

* Different value types, not just integers
* Table indexes
* Functions
* Aggregates
* Persistence (by writing logs to disk)
* Snapshots (to optimise the log or to make read-only copies)
* Transactions (multiple options here - have to figure out which is simplest)

And even start looking at research problems:

* [Cracking](http://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=1&ved=0CB8QFjAA&url=http%3A%2F%2Fstratos.seas.harvard.edu%2Ffiles%2FIKM_CIDR07.pdf&ei=KWHvVJyMHraJsQTohoGYCg&usg=AFQjCNEaHn-GFK-KtGivsTsCSmj5a8EpPA&sig2=Qw6Bo00GqS_onoFutQlr9w&bvm=bv.86956481,d.cWc) (a way to build indexes lazily and adaptively instead of making the user choose them up front)
* [Resolution](http://arxiv.org/abs/1404.0703) (a recently developed optimisation for our join algorithm that has not yet been fully explored)
* Incremental maintenance (there are lots of different techniques we could work on here)

I'll python-ish pseudocode for all the algorithms. It's just a guide so don't feel that you have to follow the same design or API, do whatever seems natural in your language.

## Data model

To start off with, all the values in our database will be integers that fall between some predefined minimum and maximum value. Choose whatever bounds you like.

``` python
LEAST = -1000
GREATEST 1000
```

Each table has only one column, so is just an array of integers:

``` python
foo = [0, 13, 6]
```

The tables have the following API:

``` python
def sort(table):
# sorts the table in numerical order - beware of eg javascript which defaults to alphabetical order

sort(foo)
# => [0, 6, 13]

def next(table, value, inclusive):
# if inclusive=True, find the first row in the table which is >= value
# if inclusive=False, find the first row in the table which is > value
# if there is such a row, returns the row
# otherwise, returns GREATEST
# only has to work on sorted tables (so you can use binary search if you want)

next(foo, value=0, inclusive=True)
# => 0

next(foo, value=0, inclusive=False)
# => 6

next(foo, value=9, inclusive=True)
# => 13

next(foo, value=9, inclusive=False)
# => 13

next(foo, value=13, inclusive=True)
# => 13

next(foo, value=13, inclusive=False)
# => GREATEST
```

## Simple joins

We have some number of tables which we want to join together:

``` python
join([[0, 13, 6]
      [6, 13, 7, 0, 11, 23]
      [109, 6, 7, 13]])
# => [6, 13]
```

There are plenty of obvious ways to do this. We are going to use a algorithm that may seem needlessly complicated now, but we will be able to extend it into a full relational query engine.

We start off with a hopelessly naive algorithm that checks every possible value:

``` python
def join(tables):
  results = []
  value = LEAST
  while True:
    value += 1
    if (value == GREATEST):
      break
    if all([contains(table, value) for table in tables]):
      results.append(value)
  return results
```

But we don't actually have to check all the possible values one by one. We can use the `next` function we wrote earlier to see how far we can skip ahead:

``` python
def join(tables):
  results = []
  for table in tables:
    sort(table)
  value = LEAST
  while True:
    nexts = [next(table, value, inclusive=True) for table in tables]
    value = max(nexts)
    if (value == GREATEST):
      break
    if all([next == value for next in nexts]):
      results.append(value)
      # get away from this value
      nexts = [next(table, value, inclusive=False) for table in tables]
      value = min(nexts)
      if (value == GREATEST):
        break
  return results
```

Note: We could easily combine `contains` and `next` into a single function. I keep them separate here to make the example code clearer.

Later on, when we start handling multiple columns, the number of possible combinations we have to look at will explode and this algorithm will let us skip past huge chunks. Even later on, we will build indexes that keep their contents in sorted order so we don't have to call `sort` for every join.

# More types

Working with integers alone is boring. We can use whatever values we want, so long as:

* there is some [total ordering](http://en.wikipedia.org/wiki/Total_order) on all the values
* there is a LEAST value that is LESS_THAN every other value and is not used in the database
* there is a GREATEST value that is GREATER_THAN every other value and is not used in the database

Note that the built-in ordering in most languages either doesn't work on values of different types or is not a total order eg in javascript:

``` js
50 < "a"
// => false
"a" < 50
// => false
50 > "a"
// => false
```

You may need to write your own comparison function if you are writing in such a language:

``` js
// SUPERHACK - values can be numbers or strings

var least = false; // 'bool' < 'number', 'string'
var greatest = undefined; // 'number', 'string' < 'undefined'

function compareValue(a, b) {
  if (a === b) return 0;
  var at = typeof a;
  var bt = typeof b;
  if ((at === bt && a < b) || (at < bt)) return -1;
  return 1;
}
```

Some languages can derive the correct function if you wrap your values eg in ocaml:

``` ocaml
type value =
  | Least
  | Int of int
  | String of string
  | Greatest

compare (Int 50) (String "a")
(* => -1 *)

compare (String "a") (Int 50)
(* => 1 *)
```

Use these comparison functions in the previous join algorithm when sorting, checking equality and finding maximums.

# Lexicographic ordering

A quick diversion - we need a way to compare whole rows of values. The ordering we will use is [colexicographic order](http://en.wikipedia.org/wiki/Lexicographical_order#Colexicographic_order). Despite the scary name, this just means we compare the first column of each row, then the second and so on until we find a column where they are different:

``` python
def compare_colexicographic(row_a, row_b):
  assert(len(row_a) == len(row_b))
  for column in range(0, len(row_a)):
    comparison = compare(row_a[column], row_b[column])
    if (comparison == LESS_THAN) return LESS_THAN
    if (comparison == GREATER_THAN) return GREATER_THAN
  return EQUAL

compare_colexicographic((0,0,0),(0,0,0))
# => EQUAL

compare_colexicographic((0,0,0),(0,0,1))
# => LESS_THAN

compare_colexicographic((0,0,0),(0,0,-1))
# => GREATER_THAN

compare_colexicographic((0,0,1),(0,1,0))
# => LESS_THAN

compare_colexicographic((0,0,-1),(0,-1,0))
# => GREATER_THAN
```

# Complex joins

This is where things start to get interesting. We are going to use almost the same join algorithm as before to handle complex queries. Our tables are now arrays of tuples:

[("ham", 0, 7.5), ("eggs", 1, 92.0), ("chips", 2, 100.01)]

We will assume that there are no duplicate tuples, and worry about how to enforce that later on.

The queries are going to be expressed in a slightly awkward way (later on we will make a query compiler to hide this). Say we have the following tables:

```
users(email, id)
logins(id, ip)
bans(ip)
```

And we want to evaluate this query:

```
(users.id = logins.id) && (logins.ip = bans.ip)
```

We now group together all the columns that were joined, and give each group a number. These are the variables for our query.

```
0: user.id, logins.id
1: logins.ip, bans.ip
2: users.email
```

Now for each table we write down the variable number for each column:

``` python
 {'users': (2, 0)
  'logins': (0, 1)
  'bans': (1)}
```

We need to be able to unmap from the variables back to rows in each table:

``` python
unmap(("a","b","c"), mapping=(0, 1, 2))
# => ("a","b","c")

unmap(("a","b","c"), mapping=(1, 2))
# => ("b","c")

unmap(("a","b","c"), mapping=(2, 0))
# => ("c","a")

unmap(("a","b","c"), mapping=(1))
# => ("b")
```

Now that we have multiple columns we have to do all our sorting, searching and nexting in colexicographic order.

``` python
users = [("a@a", 0), ("c@c", 2), ("b@b", 3), ("b@b", 4)]

sort(users)
# => [("a@a", 0), ("b@b", 3), ("b@b", 4), ("c@c", 2)]

next(users, prevRow=(LEAST, LEAST), inclusive=True)
# => ("a@a", 0)

next(users, prevRow=(LEAST, LEAST), inclusive=False)
# => ("a@a", 0)

next(users, prevRow=("a@a", 0), inclusive=True)
# => ("a@a", 0)

next(users, prevRow=("a@a", 0), inclusive=False)
# => ("b@b", 2)

next(users, prevRow=("c@c", 2), inclusive=True)
# => ("c@c", 2)

next(users, prevRow=("c@c", 2), inclusive=False)
# => (GREATEST, GREATEST)
```

The join algorithm is going to look at all the possible combinations of values that could go in those variables in colexicographic order, starting with:

``` python
value = (LEAST, LEAST, LEAST)
```

We have to be careful when figuring out how far to skip - it can be pretty tricky:

``` python
remap(nextRow=("b@b", 1), variables=("a@a", 0, 0), mapping=[0, 1])
# => ("b@b", 1, LEAST) -- we set the last column to LEAST because we have skipped on the previous columns

remap(nextRow=("b@b", 1), variables=("a@a", 0, 0), mapping=[0,2])
# => ("b@b", LEAST, LEAST) -- we can't skip on the last column because we skipped on the first and now we don't know the middle one

remap(nextRow=("b@b", 1), variables=("b@b", 0, 0), mapping=[0,2])
# => ("b@b", 0, 1) -- the first value matches, so we leave the middle column alone and only skip on the last

remap(nextRow=("b@b", 1), variables=(0, 0, "a@a"), mapping=[2,0])
# => (0, 0, "b@b") -- we can't skip to (1, 0, "b@b") because we would miss eg ("c@c", 0) so we only skip on the last column

remap(nextRow=("b@b", 1), variables=(0, 0, "b@b"), mapping=[2,0])
# => (0, 0, "b@b") -- we can't skip anything at all because we don't know if there is a ("a@a", 1) in the table
```

To make this simpler we will only allow mappings where all the variable numbers are ascending (ie `mapping=(0,2)` is ok but `mapping=(2,0)` is forbidden). Then the rule is:

* Find the first difference between variables and nextRow
* Set the corresponding variable to the value from nextRow
* Set all the remaining variables after that variable to LEAST

``` python
def remap(nextRow, variables, mapping):
  variables = variables.copy() # dont modify the original!
  for rowIndex in range(len(nextRow)):
    variableIndex = mapping[rowIndex]
    if (nextRow[rowIndex] !== variables[variableIndex]):
      variables[variableIndex] = nextRow[rowIndex]
      for lesservariableIndex in range(variableIndex+1, len(variables)):
        variables[lesservariableIndex] = LEAST
      break
  return variables
```

We can wrap all of this up:

``` python
class RowClause:
  def __init__(self, table, mapping):
    self.table = table;
    self.mapping = mapping;

  def init():
    sort(self.table)

  def search(self, variables)
    row = unmap(variables, self.mapping)
    return search(self.table, row)

  def next(self, variables):
    prevRow = unmap(variables, self.mapping)
    nextRow = next(self.table, prevRow)
    return remap(nextRow, variables, self.mapping)
```

And we finally the join algorithm itself is almost exactly the same as the single-column case:

``` python
def join(numVariables, clauses):
  for clause in clauses:
    clause.init()
  variables = (LEAST,) * numVariables
  results = []
  while True:
    nexts = [clause.next(variables, inclusive=True) for clause in clauses]
    variables = max(nexts) # maximum by compare_colexicographic!
    if variables[0] == GREATEST:
      # no more results
      break
    if all([next == variables for next in nexts]) :
      results.append(variables.copy())
      # get away from these variables
      nexts = [clause.next(variables, inclusive=False) for clause in clauses]
      variables = min(nexts) # minimum by compare_colexicographic!
      if variables[0] == GREATEST:
        # no more results
        break

users = [("a@a", 0), ("c@c", 2), ("b@b", 3), ("b@b", 4)]
logins = [(2, "0.0.0.0"), (2, "1.1.1.1"), (4, "1.1.1.1")]
bans = [("1.1.1.1",), ("2.2.2.2,")]
join(3, [RowClause(users, (2, 0)), RowClause(logins, (0, 1)), RowClause(bans, (1,))])
# => [(2, "1.1.1.1", "c@c"), (4, "b@b", "1.1.1.1")]
```

