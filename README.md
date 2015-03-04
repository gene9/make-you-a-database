TODO

* Switch from pseudocode to real (tested) code?
* Add a proof for the join algorithm, or at least talk about why it works
* Basic join optimisations
* Query compiler
* Talk about relationship between variable ordering, selectivity, skew etc
* Describe relation to [Tetris algorithm](http://arxiv.org/abs/1404.0703)
* More features
* Figure out how to organise optional/branching material
* Figure out how to version example code for various branches

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

Say we have the following multi-column tables:

```
users(email, id)
logins(id, ip)
bans(ip)
```

And we want to evaluate this query:

```
(users.id = logins.id) && (logins.ip = bans.ip)
```

This is where things start to get interesting. We are going to use almost exactly the same algorithm as before. The only complication is that our tables are now arrays of tuples like:

``` python
users = [
  (0, "a@a"),
  (2, "c@c"),
  (3, "b@b"),
  (4, "b@b")
]

logins = [
  (2, "0.0.0.0"),
  (2, "1.1.1.1"),
  (4, "1.1.1.1")
]

bans = [
  ("1.1.1.1",),
  ("2.2.2.2,")
]
```

And the results of our query are also arrays of tuples:

``` python
# (user.id, users.email, logins.id, logins.ip, bans.ip, )
results = [
  (2, "c@c", 2, "1.1.1.1", "1.1.1.1"),
  (4, "b@b", 4, "1.1.1.1", "1.1.1.1")
]
```

As you can see, each columns of each table is included in the results. But since our query is `(users.id = logins.id) && (logins.ip = bans.ip)` we know that the `users.id` column and the `logins.id` column will both have the same value in the results, and similarly for the `logins.ip` column and the `bans.ip` column. So we will remove those duplicates:

``` python
# (user.id/logins.id, users.email, logins.ip/bans.ip)
results = [
  (2, "c@c", "1.1.1.1"),
  (4, "b@b", "1.1.1.1")
]
```

For now we can express our query to the join solver by specifying where each column ends up in the output:

``` python
join(3, # 3 unique columns in the result
[
  RowClause(users, (1, 0)), # users[0] => results[1], users[1] => results[0]
  RowClause(logins, (0, 2)), # users[0] => results[0], users[1] => results[2]
  RowClause(bans, (2,)) # bans[0] => results[2]
])
```

Later we will make a query compiler that lets us write the nice query we had earlier and produces this column mapping for us.

The top level algorithm for join is very similar to the simple one column case. We start at the smallest possible result row `(LEAST, LEAST, LEAST)` and work our way through all the results in order, using `clause.next` to figure out how far ahead we can safely skip:

``` python
def join(numResultColumns, clauses):
  result = (LEAST,) * numResultColumns
  results = []
  while True:
    next_result = [clause.next(result, inclusive=True) for clause in clauses]
    result = max(nexts) # maximum by compare_colexicographic!
    if result[0] == GREATEST:
      # no more results
      break
    if all([next == result for next in nexts]) :
      results.append(result)
      # get away from these result
      nexts = [clause.next(result, inclusive=False) for clause in clauses]
      result = min(nexts) # minimum by compare_colexicographic!
      if result[0] == GREATEST:
        # no more results
        break
```

`clause.next` is where all the magic happens. Let's take a look:

``` python
class RowClause:
  def __init__(self, table, mapping):
    sort(table)
    self.table = table;
    self.mapping = mapping;

  def next(self, result, inclusive):
    prevRow = unmap(result, self.mapping)
    nextRow = next(self.table, prevRow, inclusive)
    return remap(nextRow, result, self.mapping)
```

We have three helper functions, `unmap`, `next` and `remap`.

`unmap` takes a result row and figures out what the corresponding table row is

``` python
unmap((4, "b@b", "1.1.1.1"), mapping=(1, 0))
# => ("b@b", 4)

unmap((4, "b@b", "1.1.1.1"), mapping=(0, 2))
# => (4, "1.1.1.1")

unmap((4, "b@b", "1.1.1.1"), mapping=(2))
# => ("1.1.1.1",)
```

`next` works just like the old, single-column version except that it searches through rows using the colexicographic order:

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

`remap` looks at the result of `next` and figures out how far ahead we can skip. This is the only tricky part. Let's take a look at some examples first:

``` python
remap(nextRow=("b@b", 1), result=("a@a", 0, 0), mapping=[0,1])
# => ("b@b", 1, LEAST) -- we have to change the last column to LEAST because we can't skip eg ("b@b", 1, -1)

remap(nextRow=("b@b", 1), result=("a@a", 0, 0), mapping=[0,2])
# => ("b@b", LEAST, 1) -- we have to change the middle column to LEAST because we can't skip eg ("b@b", -1, 1)

remap(nextRow=("b@b", 1), result=("b@b", 0, 0), mapping=[0,2])
# => ("b@b", 0, 1) -- the first value matches, so we can the middle column alone and only change the last column

remap(nextRow=("b@b", 1), result=(0, 0, "a@a"), mapping=[2,0])
# => (0, 0, "b@b") -- we can't skip to (1, 0, "b@b") because we would miss eg (0, 0, "c@c") so we can only change the last column

remap(nextRow=("b@b", 1), result=(0, 0, "b@b"), mapping=[2,0])
# => (0, 0, "b@b") -- we can't skip anything at all because we don't know if there is a ("a@a", 1) in the table
```

To make this simpler we will only allow mappings where the column numbers are ascending (ie `mapping=(0,2)` is ok but `mapping=(2,0)` is forbidden). For some queries we might have to change the order of the columns in the input tables to find an allowed mapping - this will be part of the query compilers job later on.

For ascending mappings the rule is just:

* Find the first column where there is a difference between result and nextRow
* Set all the remaining columns after that column to LEAST
* Copy over all the columns from nextRow

You can see that this rule works for the first three examples above and doesn't allow the last two examples.

``` python
def remap(nextRow, result, mapping):
  result = result.copy() # dont modify the original!
  for rowColumn in range(len(nextRow)):
    resultColumn = mapping[rowColumn]
    if (nextRow[rowColumn] !== result[resultColumn]):
      for lesserColumn in range(resultColumn+1, len(result)):
        result[lesserColumn] = LEAST
      break
  for rowColumn in range(len(nextRow)):
    resultColumn = mapping[rowColumn]
    result[resultColumn] = nextRow[rowColumn]
  return result
```

Now we have a join algorithm that can evaluate arbitarily complex join on multi-column tables.
