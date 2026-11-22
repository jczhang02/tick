# Sync: User item orders

_A user item order places one task in one of the user's personal orderings.
Each object is one task's position in one ordering, held as a fractional-indexing
`order_key` rather than an integer: rows sort by comparing keys
lexicographically within a scope. These orderings are private to the user and
are not shared with a project's collaborators._

_Today only the `day` scope is served: the ordering behind the Today and
Upcoming views, one key space per user with no date. It is the keyed
replacement for [day orders](https://developer.todoist.com/api/v1/sync.md), which remain
available and stay in step._

> An example user item order:

```
{
    "item_id": "6X7rM8997g3RQmvh",
    "scope": "day",
    "scope_id": 0,
    "order_key": "a3",
    "is_deleted": false
}
```

#### Properties

| Property             | Description                                                                                                                  |
|----------------------|------------------------------------------------------------------------------------------------------------------------------|
| item_id _String_     | The ID of the task this ordering entry places.                                                                               |
| scope _String_       | Which of the user's orderings this entry belongs to. Only `day` is currently served.                                         |
| scope_id _Integer_   | The instance of that scope. Always `0` for `day`, which is a single key space per user.                                       |
| order_key _String_   | The fractional-indexing order key: entries sort by comparing keys lexicographically within a scope.                          |
| is_deleted _Boolean_ | Whether the entry has been cleared (a `true` or `false` value). A cleared entry carries a `null` `order_key`.                |

## Update a user item order

> Example update user item order request:

```shell
$ curl https://api.todoist.com/api/v1/sync \
    -H "Authorization: Bearer 0123456789abcdef0123456789abcdef01234567" \
    -d commands='[
    {
        "type": "user_item_order_update",
        "uuid": "e1f0d5c4-3b2a-4190-8d7e-6c5b4a392817",
        "args": {"item_id": "6X7rM8997g3RQmvh", "scope": "day", "order_key": "a3"}
    }]'
```

> Example response:

```shell
{
  ...
  "sync_status": {"e1f0d5c4-3b2a-4190-8d7e-6c5b4a392817": "ok"},
  ...
}
```

Place one task in one of the user's orderings, or clear its place.

The command writes exactly one entry. A sync request already batches commands,
so reordering several tasks means sending several of these.

If the supplied `order_key` is already used by another entry in the same
ordering, the server does not reject the command: it places the task
immediately after the requested position and returns the resolved key in the
next sync. Commands for a task the user cannot see, or that has been deleted,
are dropped and still report `ok`.

#### Command arguments

| Argument             | Required | Description                                                                                                   |
|----------------------|----------|-----------------------------------------------------------------------------------------------------------------|
| item_id _String_     | Yes      | The ID of the task to place.                                                                                  |
| scope _String_       | Yes      | Which ordering to place it in. Only `day` is currently accepted.                                              |
| order_key _String_   | Yes      | Where to place it, as a fractional-indexing key. Pass `null` to clear the task's place in this ordering.       |
