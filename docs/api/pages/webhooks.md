# Webhooks

The Todoist Webhooks API allows applications to receive real-time notification
(in the form of HTTP POST payload) on the subscribed user events. Notice that
once you have a webhook setup, you will start receiving webhook events from
**all your app users** immediately.

##### Important Considerations

- For security reasons, Todoist only allows webhook urls that have HTTPS enabled and no ports specified in the url.
    - Allowed webhook url:
        - `https://nice.integration.com`
    - Disallowed webhook url:
        - `http://evil.integration.com`
        - `https://bad.integration.com:5980`

- Due to the nature of network requests, your application should assume webhook
requests could arrive delayed, out of order, or could even fail to arrive at all; webhooks should
be used only as notifications and not as primary Todoist data sources (make
sure your application could still work when webhook is not available).

##### Webhook Activation & Personal Use

The webhook for a specific user is activated when that user completes the [OAuth flow](https://developer.todoist.com/api/v1/authorization.md) of the app that declares the webhook.

**Todoist webhooks don't fire by default for the user that has created the Todoist app, which is frequently the desired state for the personal use of webhooks.**

To activate webhooks for personal use, you need to complete the OAuth process with your account. You can do this without code by manually executing the OAuth flow in two steps.

1. Performing the [authorization request](https://developer.todoist.com/api/v1/authorization.md) in the browser and capturing the `code` via the browser's developer tools.
2. Performing the [token exchange request](https://developer.todoist.com/api/v1/authorization.md) through a tool like [Postman](https://www.postman.com/) and reading the `access_token` from the response. _Note that you can't make this request via the browser as it needs to be a POST request._

### Configuration

Before you can start receiving webhook event notifications, you must first have
your webhook configured at the App Management Console.

##### Events

Here is a list of events that you could subscribe to, and they are configured at
the [App Management Console](https://app.todoist.com/app/settings/integrations/app-management).

| Event Name         | Description              | Event Data                             |
| ------------------ | ------------------------ | -------------------------------------- |
| item:added         | A task was added        | The new [Task](https://developer.todoist.com/api/v1/sync/tasks.md).                |
| item:updated       | A task was updated      | The updated [Task](https://developer.todoist.com/api/v1/sync/tasks.md).            |
| item:deleted       | A task was deleted      | The deleted [Task](https://developer.todoist.com/api/v1/sync/tasks.md).            |
| item:completed     | A task was completed    | The completed [Task](https://developer.todoist.com/api/v1/sync/tasks.md).          |
| item:uncompleted   | A task was uncompleted  | The uncompleted [Task](https://developer.todoist.com/api/v1/sync/tasks.md).        |
| note:added         | A comment was added         | The new [Comment](https://developer.todoist.com/api/v1/sync/comments.md).                |
| note:updated       | A comment was updated       | The updated [Comment](https://developer.todoist.com/api/v1/sync/comments.md).            |
| note:deleted       | A comment was deleted       | The deleted [Comment](https://developer.todoist.com/api/v1/sync/comments.md).            |
| project:added      | A project was added      | The new [Project](https://developer.todoist.com/api/v1/sync/projects.md).          |
| project:updated    | A project was updated    | The updated [Project](https://developer.todoist.com/api/v1/sync/projects.md).      |
| project:deleted    | A project was deleted    | The deleted [Project](https://developer.todoist.com/api/v1/sync/projects.md).      |
| project:archived   | A project was archived   | The archived [Project](https://developer.todoist.com/api/v1/sync/projects.md).     |
| project:unarchived | A project was unarchived | The unarchived [Project](https://developer.todoist.com/api/v1/sync/projects.md).   |
| section:added      | A section was added      | The new [Section](https://developer.todoist.com/api/v1/sync/sections.md).          |
| section:updated    | A section was updated    | The updated [Section](https://developer.todoist.com/api/v1/sync/sections.md).      |
| section:deleted    | A section was deleted    | The deleted [Section](https://developer.todoist.com/api/v1/sync/sections.md).      |
| section:archived   | A section was archived   | The archived [Section](https://developer.todoist.com/api/v1/sync/sections.md).     |
| section:unarchived | A section was unarchived | The unarchived [Section](https://developer.todoist.com/api/v1/sync/sections.md).   |
| label:added        | A label was added        | The new [Label](https://developer.todoist.com/api/v1/sync/labels.md).              |
| label:deleted      | A label was deleted      | The deleted [Label](https://developer.todoist.com/api/v1/sync/labels.md).          |
| label:updated      | A label was updated      | The updated [Label](https://developer.todoist.com/api/v1/sync/labels.md).          |
| filter:added       | A filter was added       | The new [Filter](https://developer.todoist.com/api/v1/sync/filters.md).            |
| filter:deleted     | A filter was deleted     | The deleted [Filter](https://developer.todoist.com/api/v1/sync/filters.md).        |
| filter:updated     | A filter was updated     | The updated [Filter](https://developer.todoist.com/api/v1/sync/filters.md).        |
| reminder:fired     | A reminder has fired     | The [Reminder](https://developer.todoist.com/api/v1/sync/reminders.md) that fired. |

##### Events Extra

Some events can include extra meta information in the `event_data_extra` field. These can be useful, for example, if you need to distinguish between item updates that are postponed and initiated by the user and item updates that are task completions (initiated by completing a recurring task)

| Event Name   | Description                                                                         | Event Data                                                                                                             |
| ------------ | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| item:updated | For events issued by the user directly these include `old_item` and `update_intent` | `old_item` will be an [Task](https://developer.todoist.com/api/v1/sync/tasks.md), and `update_intent` can be `item_updated`, `item_completed`, `item_uncompleted`. |

### Request Format

##### Event JSON Object

> Example Webhook Request

```text
POST /payload HTTP/1.1

Host: your_callback_url_host
Content-Type: application/json
X-Todoist-Hmac-SHA256: UEEq9si3Vf9yRSrLthbpazbb69kP9+CZQ7fXmVyjhPs=
```

```json
{
    "event_name": "item:added",
    "user_id": "2671355",
    "event_data": {
        "added_by_uid": "2671355",
        "assigned_by_uid": null,
        "checked": false,
        "child_order": 3,
        "collapsed": false,
        "content": "Buy Milk",
        "description": "",
        "added_at": "2025-02-10T10:33:38.000000Z",
        "completed_at": null,
        "due": null,
        "deadline": null,
        "id": "6XR4GqQQCW6Gv9h4",
        "is_deleted": false,
        "labels": [],
        "parent_id": null,
        "priority": 1,
        "project_id": "6XR4H993xv8H5qCR",
        "responsible_uid": null,
        "section_id": null,
        "url": "https://app.todoist.com/app/task/6XR4GqQQCW6Gv9h4",
        "user_id": "2671355"
    },
    "initiator": {
        "email": "alice@example.com",
        "full_name": "Alice",
        "id": "2671355",
        "image_id": "ad38375bdb094286af59f1eab36d8f20",
        "is_premium": true
    },
    "triggered_at": "2025-02-10T10:39:38.000000Z",
    "version": "10"
}
```

Each webhook event notification request contains a JSON object. The event JSON
contains the following properties:

| Property                   | Description                                                                                                                                                                       |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| event_name _String_       | The event name for the webhook, see the table in the [Configuration](https://developer.todoist.com/api/v1/webhooks.md) section for the list of supported events.                                                    |
| user_id _String_          | The ID of the user that is the destination for the event.                                                                                                                         |
| event_data _Object_       | An object representing the modified entity that triggered the event, see the table in the [Configuration](https://developer.todoist.com/api/v1/webhooks.md) section for details of the `event_data` for each event. |
| version _String_           | The version number of the webhook configured in the [App Management Console](https://app.todoist.com/app/settings/integrations/app-management).                                                                                   |
| initiator _Object_         | A [Collaborator](https://developer.todoist.com/api/v1/sync/sharing.md#collaborators) object representing the user that triggered the event. This may be the same user indicated in `user_id` or a collaborator from a shared project. |
| triggered_at _String_     | The date and time when the event was triggered.                                                                                                                                   |
| event_data_extra _Object_ | Optional object that can include meta information, see the table in the [Configuration](https://developer.todoist.com/api/v1/webhooks.md) section for details of the `event_data_extra` for each event.             |

##### Request Header

| Header Name           | Description                                                                                                                                                                                                                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| User-Agent            | Will be set to "Todoist-Webhooks"                                                                                                                                                                                                                                                                              |
| X-Todoist-Hmac-SHA256 | To verify each webhook request was indeed sent by Todoist, an `X-Todoist-Hmac-SHA256` header is included; it is a SHA256 Hmac generated using your `client_secret` as the encryption key and the whole request payload as the message to be encrypted. The resulting Hmac would be encoded in a base64 string. |
| X-Todoist-Delivery-ID | Each webhook event notification has a unique `X-Todoist-Delivery-ID`. When a notification request failed to be delivered to your endpoint, the request would be re-delivered with the same `X-Todoist-Delivery-ID`.                                                                                            |

##### Failed Delivery

When an event notification fails to be delivered to your webhook callback URL
(i.e. due to server / network error, incorrect response, etc),
it will be reattempted after 15 minutes. Each notification will be
reattempted for at most three times.

**Your callback endpoint must respond with an HTTP 200 when receiving an event
notification request.**

A response other than HTTP 200 will be considered as a failed delivery, and the
notification will be attempted again.
