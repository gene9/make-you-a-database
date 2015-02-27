State of this guide

* Basic outline is in place
* Algorithms need to be tweaked to make later additions easier
* Multi-column join is not yet tested

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

Each table will be represented as an array of tuples:

``` python
foo = [(0, 0, 0), (0, 13, 1), (5, 6, 2)]
```

For now we will assume that every tuple is unique. Later on we will worry about how to enforce that.

The tables have the following API:

``` python
def sort(table, column):
# sorts the table by the specified column

sort(foo, column=1)
# => [(0,0,0), (5,6,2), (0,13,1)]

def next(table, column, value, inclusive):
# if inclusive=True, find the first row in the table which has row[column] >= value
# if inclusive=False, find the first row in the table which has row[column] > value
# if there is such a row, returns the row
# otherwise, returns GREATEST for each column
# only has to work on sorted tables (so you can use binary search if you want)

next(foo, column=1, value=0, inclusive=True)
# => (0,0,0)

next(foo, column=1, value=0, inclusive=false)
# => (5,6,2)

next(foo, column=1, value=9, inclusive=True)
# => (0,13,1)

next(foo, column=1, value=9, inclusive=false)
# => (0,13,1)

next(foo, column=1, value=13, inclusive=True)
# => (0,13,1)

next(foo, column=1, value=13, inclusive=false)
# => (GREATEST, GREATEST, GREATEST)
```

Be careful with the sorting, in some languages the default comparison may not do what you expect eg in javascript:

``` js
x = [3,200,10]
x.sort()
// => [10, 200, 3]
```

## Single-column join

We have some number of tables which we want to join together:

``` python
join({'foo': [(0, 0, 0), (0, 13, 1), (5, 6, 2)]
      'bar': [(6,), (13,), (7,), (0,), (11,), (23,)]
      'quux': [(109, 109), (101, 6), (102, 7), (103, 13)]},
     [('foo', 1), ('bar', 0), ('quux', 1)])
# join foo, bar, quux where foo[1] = bar[0] = quux[1]
# => [(5, 6, 2, 6, 101, 6), (0, 13, 1, 13, 103, 13)]
# notice that we just mash the rows together in the output
# we will handle 'select' later on
```

There are plenty of obvious ways to do this. We are going to use a algorithm that may seem needlessly complicated now, but we will be able to extend it into a full relational query engine.

We start off with a hopelessly naive algorithm that checks every possible value:

``` python
value = LEAST
while True:
  if (hasValue(foo, column=1, value) and hasValue(bar, column=0, value) and hasValue(quux, column=1, value)):
    results.append(value)
  if (value == GREATEST):
    break
  value += 1
# => [6, 13]
```

But we don't actually have to check all the possible values one by one. We can use the `next` function we wrote earlier to see how far we can skip ahead:

``` python
sort(foo, column=1)
sort(bar, column=0)
sort(quux, column=1)
value = LEAST
results = []
while True:
  fooRow = next(foo, column=1, value, inclusive=True)
  barRow = next(bar, column=0, value, inclusive=True)
  quuxRow = next(quux, column=1, value, inclusive=True)
  if (fooRow[1] === GREATEST) or (barRow[0] === GREATEST) or (quuxRow[1] === GREATEST):
    # no more solutions - one of our tables ran out
  	break
  value = max(fooRow[1], barRow[0], quuxRow[1])
  if (fooRow[1] == barRow[0] == quuxRow[1]):
    # we found a solution!
    results.append(fooRow + barRow + quuxRow)
    # skip to the next value - note inclusive=False
    fooRow = next(foo, column=1, value, inclusive=False)
    barRow = next(bar, column=0, value, inclusive=False)
    quuxRow = next(quux, column=1, value, inclusive=False)
    value = max(fooRow[1], barRow[0], quuxRow[1])
    if (fooRow[1] === GREATEST) or (barRow[0] === GREATEST) or (quuxRow[1] === GREATEST):
      # no more solutions - one of our tables ran out
  	  break
```

All you have to do now is make this work for arbitrary inputs like the one shown earlier:

``` python
join({'foo': [(0, 0, 0), (0, 13, 1), (5, 6, 2)]
      'bar': [(6,), (13,), (7,), (0,), (11,), (23,)]
      'quux': [(109, 109), (101, 6), (102, 7), (103, 13)]},
     [('foo', 1), ('bar', 0), ('quux', 1)])
```

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

compare (Int 50) (String "a") (* => -1 *)
compare (String "a") (Int 50) (* => 1 *)
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

compare_colexicographic((0,0,0),(0,0,0)) # => EQUAL
compare_colexicographic((0,0,0),(0,0,1)) # => LESS_THAN
compare_colexicographic((0,0,0),(0,0,-1)) # => GREATER_THAN
compare_colexicographic((0,0,1),(0,1,0)) # => LESS_THAN
compare_colexicographic((0,0,-1),(0,-1,0)) # => GREATER_THAN
```

# Multiple columns

This is where things start to get interesting. We are going to use essentially the same join algorithm as before with a few tweaks to handle complex joins.

We are going to express joins in a slightly awkward way. Say we have the following tables:

```
users(email, id)
logins(id, ip)
bans(ip)
```

And we want to evaluate this query:

```
(users.id = logins.id) && (logins.ip = bans.ip)
```

We figure out the number of *unique* columns that will be in the result and assign an slot to each one. We can map them in any order, as long as all the columns that are joined together end up in the same slot.

```
0: user.id, logins.id
1: logins.ip, bans.ip
2: users.email
```

Now for each table we write down which slots it's columns end up in:

``` python
 {'users': (2, 0)
  'logins': (0, 1)
  'bans': (1)}
```

Later on we will look at how the performance is affected by the order of the slots and the order of the columns in each table, but for now just pick any order.

We need to be able to unmap from the slots back to rows in each table:

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

The `next` function now needs to sort and search rows in colexicograpic order, not just by one column:

``` python
users = [("a@a", 0), ("c@c", 2), ("b@b", 3), ("b@b", 4)]

users.sort(compare_colexicographic)
# => [("a@a", 0), ("b@b", 3), ("b@b", 4), ("c@c", 2)]

next(users, prevRow=(LEAST, LEAST), inclusive=True)
# => ("a@a", 0)

next(users, prevRow=("a@a", 0), inclusive=True)
# => ("a@a", 0)

next(users, prevRow=("a@a", 0), inclusive=False)
# => ("b@b", 2)

next(users, prevRow=("a@a", 7), inclusive=True)
# => ("b@b", 2)

next(users, prevRow=("c@c", 1), inclusive=True)
# => ("c@c", 2)

next(users, prevRow=("c@c", 3), inclusive=True)
# => (GREATEST, GREATEST)
```

The join algorithm is going to look at all the possible combinations of values that could go in those slots in colexicographic order, starting with:

``` python
value = (LEAST, LEAST, LEAST)
```

Now for the trick part - we have to be careful when figuring out how far to skip:

``` python
remap(nextRow=("b@b", 1), slots=("a@a", 0, 0), mapping=[0, 1])
# => ("b@b", 1, LEAST) -- we set the last column to LEAST because we have skipped on the previous columns

remap(nextRow=("b@b", 1), slots=("a@a", 0, 0), mapping=[0,2])
# => ("b@b", LEAST, LEAST) -- we can't skip on the last column because we skipped on the first and now we don't know the middle one

remap(nextRow=("b@b", 1), slots=("b@b", 0, 0), mapping=[0,2])
# => ("b@b", 0, 1) -- the first value matches, so we leave the middle column alone and only skip on the last

remap(nextRow=("b@b", 1), slots=("b@b", 0, 1), mapping=[0,2])
# => ("b@b", 0, 1) -- the values all match, so we just leave it alone

remap(nextRow=("b@b", 1), slots=(0, 0, "a@a"), mapping=[2,0])
# => (0, 0, "b@b") -- we can't skip to (1, 0, "b@b") because we would miss eg ("c@c", 0) so we only skip on the last column

remap(nextRow=("b@b", 1), slots=(0, 0, "b@b"), mapping=[2,0])
# => ? -- we don't even know what to skip to because we don't know what other rows look like (_, 0)
```

To make this simpler we will only allow mappings where all the slot numbers are ascending (ie mapping=(0,2) is ok but mapping=(2,0) is forbidden). Then the rule is:
* Find the first difference between slots and nextRow
* Set the corresponding slot to the value from nextRow
* Set all the remaining slots after that slot to LEAST

``` python
def remap(nextRow, slots, mapping):
  slots = slots.copy() # dont modify the original!
  for rowIndex in range(len(nextRow)):
    slotIndex = mapping[rowIndex]
    if (nextRow[rowIndex] !== slots[slotIndex]):
      slots[slotIndex] = nextRow[rowIndex]
      for lesserSlotIndex in range(slotIndex+1, len(slots)):
        slots[lesserSlotIndex] = LEAST
      break
  return slots
```

TODO test this code :)

We can wrap all of this up:

``` python
class RowClause:
  def __init__(self, table, mapping):
    self.table = table;
    self.mapping = mapping;

  def init():
    self.table.sort(compare_colexicographic)

  def next(self, slots, inclusive):
    prevRow = unmap(slots, self.mapping)
    nextRow = next(table, prevRow, inclusive)
    return remap(nextRow, slots, self.mapping)
```

And we finally the join algorithm itself is almost exactly the same as the single-column case:

``` python
def join(numSlots, clauses):
  for clause in clauses:
    clause.init()
  slots = (LEAST,) * numSlots
  results = []
  while True:
    nextSlots = [clause.next(slots, inclusive=True) for clause in clauses]
    slots = max(nextSlots) # maximum by compare_colexicographic!
    if slots[0] == GREATEST:
      # no more results
      break
    if allEqual(nextSlots):
      results.append(slots.copy())
      nextSlots = [clause.next(slots, inclusive=False) for clause in clauses]
      slots = max(nextSlots) # maximum by compare_colexicographic!
      if slots[0] == GREATEST:
        # no more results
        break


users = [("a@a", 0), ("c@c", 2), ("b@b", 3), ("b@b", 4)]
logins = [(2, "0.0.0.0"), (2, "1.1.1.1"), (4, "1.1.1.1")]
bans = [("1.1.1.1",), ("2.2.2.2,")]
join(3, [RowClause(users, (2, 0)), RowClause(logins, (0, 1)), RowClause(bans, (1,))])
# =>
```

