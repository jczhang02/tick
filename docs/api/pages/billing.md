# Billing

## Endpoints

### Usage Billing Usage Summary

`GET /api/v1/usage_billing/usage_summary`

Total the caller's settled usage over a window.

`/history` is a ledger with one row per movement, so a total spanning more
than one page cannot come from it. This answers the one question the UI asks
of the whole ledger at once: how many runs, and what they cost.

Only settled usage counts. `void`, `expiry` and `chargeback` draw the balance
down as well, but they reverse a grant rather than report usage.

A user who has never topped up reports zero for every field.

`group_by=automation` adds `groups`: the same total, one entry per
automation. Older usage without an automation id groups by its label.
`group_by=user` does the same per member, for a workspace's wallet.

With `workspace_id`, total that workspace's usage. Only its admins can: a
member who isn't an admin gets `FORBIDDEN` (403).

```json
{
  "parameters": [
    {
      "name": "product",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "description": "Product whose usage to total.",
        "examples": [
          "automations"
        ],
        "default": "automations",
        "title": "Product"
      }
    },
    {
      "name": "category",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "Count one kind of work only, for example `automation_run`. More than one kind settles against the same balance, so a per-run average over all of them mixes runs with chat messages. Omit to total every category.",
        "examples": [
          "automation_run"
        ],
        "title": "Category"
      }
    },
    {
      "name": "since",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "format": "date-time"
          },
          {
            "type": "null"
          }
        ],
        "description": "Start of the window, inclusive. Omit to start at the account's first event. A value with no offset is read as UTC, which is what `created_at` in `/history` reports.",
        "examples": [
          "2026-07-15T00:00:00Z"
        ],
        "title": "Since"
      }
    },
    {
      "name": "until",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "format": "date-time"
          },
          {
            "type": "null"
          }
        ],
        "description": "End of the window, exclusive. Omit to run to now. A value with no offset is read as UTC.",
        "examples": [
          "2026-08-15T00:00:00Z"
        ],
        "title": "Until"
      }
    },
    {
      "name": "group_by",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "enum": [
              "automation",
              "user"
            ],
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "Split the total in `groups`. `automation`: by the automation that ran. Send `category=automation_run` with it: chat messages carry no automation, so without the filter each chat label becomes a group of its own. `user`: by the member the usage was for, on a workspace's wallet; it needs `workspace_id`, and without it is a 400.",
        "examples": [
          "automation",
          "user"
        ],
        "title": "Group By"
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer",
            "maximum": 18446744073709551615,
            "minimum": 1
          },
          {
            "type": "null"
          }
        ],
        "description": "The workspace whose wallet this is about. Omit for the caller's personal wallet. An unknown workspace, or one the caller isn't an admin or member of, is `WORKSPACE_NOT_FOUND` (404); guests count as non-members.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_UsageSummaryResponse"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Usage Billing History

`GET /api/v1/usage_billing/history`

List the caller's usage-billing ledger entries, newest first.

Use `types` to filter the entries to list. A user who has never topped up
gets an empty page.

With `workspace_id`, list that workspace's ledger. Only its admins can: a
member who isn't an admin gets `FORBIDDEN` (403).

This is a paginated endpoint. See the [Pagination guide](https://developer.todoist.com/api/v1/pagination.md) for
details on using cursor-based pagination.

```json
{
  "parameters": [
    {
      "name": "product",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "description": "Product whose ledger to read.",
        "examples": [
          "automations"
        ],
        "default": "automations",
        "title": "Product"
      }
    },
    {
      "name": "types",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string"
          },
          {
            "type": "null"
          }
        ],
        "description": "Comma-separated `type` values to return: `usage_settled`, `top_up`, `adjustment`, `void`, `expiry`, or `chargeback`. When omitted, all types are returned. Rejects when an unrecognized name is supplied.",
        "examples": [
          "top_up",
          "top_up,chargeback,expiry,void"
        ],
        "title": "Types"
      }
    },
    {
      "name": "cursor",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "string",
            "minLength": 1,
            "pattern": "^[0-9a-zA-Z_-]+\\.[0-9a-zA-Z_-]+$",
            "description": "An opaque string used as the cursor for pagination. Must be used with the same parameters from the previous request",
            "examples": [
              "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
            ]
          },
          {
            "type": "null"
          }
        ],
        "title": "Cursor"
      }
    },
    {
      "name": "limit",
      "in": "query",
      "required": false,
      "schema": {
        "type": "integer",
        "maximum": 200,
        "exclusiveMinimum": 0,
        "description": "The number of objects to return in a page",
        "examples": [
          50
        ],
        "default": 50,
        "title": "Limit"
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer",
            "maximum": 18446744073709551615,
            "minimum": 1
          },
          {
            "type": "null"
          }
        ],
        "description": "The workspace whose wallet this is about. Omit for the caller's personal wallet. An unknown workspace, or one the caller isn't an admin or member of, is `WORKSPACE_NOT_FOUND` (404); guests count as non-members.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_HistoryResponse"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Usage Billing Top Up

`POST /api/v1/usage_billing/top_up`

In V1, create a hosted Stripe Checkout session to top up the authenticated user's personal Todoist Wallet.

With `workspace_id`, top up that workspace's wallet instead. Only its admins can: a member who isn't an admin gets
`FORBIDDEN` (403). The workspace needs a paid Business plan, a trial included, and pays from its Business
subscription's Stripe customer, in that subscription's currency, which `state` reports. The admin is recorded as
who paid.

A wallet whose usage is billed on the user's Pro subscription needs a personal Stripe customer and an active
personal Todoist Pro subscription. A wallet on its own compute subscription needs Pro of any kind, including Pro
gifted by a Business workspace, but not Pro paid only through an app store. A user with no Stripe customer gets one
before Checkout opens, unless they only have a legacy one.

A refused top-up returns HTTP 400 with one of these error tags:

- `USAGE_BILLING_PLAN_REQUIRED` (code `615`): the user doesn't have Pro, or the workspace has no paid Business
  plan.
- `USAGE_BILLING_CURRENCY_MISMATCH` (code `616`): the currency isn't the one the user or workspace pays in,
  including an unsupported one.
- `USAGE_BILLING_TOP_UP_BELOW_MINIMUM` (code `617`): the amount is below the minimum.
- `USAGE_BILLING_TOP_UP_UNAVAILABLE` (code `605`): any other unmet prerequisite, such as a user with only a legacy
  Stripe customer.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_b48755ce"
        }
      }
    },
    "required": true
  },
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_TopUpResponse"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Usage Billing State

`GET /api/v1/usage_billing/state`

Read the caller's usage-billing credit state for a product.

A user who has never topped up reads as an empty wallet: `available_units` and
`available_amount` are 0, `is_settled_low_balance` is true, and
`usage_block_reason` is `insufficient_credit`. There is no separate "no
account" answer, because both states send the user to the same top-up screen.

With `workspace_id`, read that workspace's wallet. Its admins see everything.
Its members see whether runs can start and whether the balance is low, so the
app can tell them, but not the amount: the four balance fields are `null`.
`currency`, `min_top_up_amount` and `top_up_presets` are the Business
subscription's.

```json
{
  "parameters": [
    {
      "name": "product",
      "in": "query",
      "required": false,
      "schema": {
        "type": "string",
        "description": "Product whose credit state to read.",
        "examples": [
          "automations"
        ],
        "default": "automations",
        "title": "Product"
      }
    },
    {
      "name": "workspace_id",
      "in": "query",
      "required": false,
      "schema": {
        "anyOf": [
          {
            "type": "integer",
            "maximum": 18446744073709551615,
            "minimum": 1
          },
          {
            "type": "null"
          }
        ],
        "description": "The workspace whose wallet this is about. Omit for the caller's personal wallet. An unknown workspace, or one the caller isn't an admin or member of, is `WORKSPACE_NOT_FOUND` (404); guests count as non-members.",
        "examples": [
          123456
        ],
        "title": "Workspace Id"
      }
    }
  ],
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/_CreditStateResponse"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Cancel Plan With Redirect To Stripe

`POST /api/v1/payments/cancel_plan_with_redirect_to_stripe`

Start a hosted cancellation flow and return the redirect URL.

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "$ref": "#/components/schemas/Body_e4b32da9"
        }
      }
    },
    "required": true
  },
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/BillingPortalUrlResponse"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Get Subscription Info

`POST /api/v1/payments/get_subscription_info`

Return the current user's subscription state.

```json
{
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SubscriptionInfo"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

### Reactivate Plan

`POST /api/v1/payments/reactivate_plan`

Reactivate a previously canceled subscription.

Returns the fresh `SubscriptionInfo` after reactivation so callers can
avoid a follow-up fetch. Raises `NO_CANCELABLE_SUBSCRIPTION` if there is
no active subscription to reactivate.

```json
{
  "responses": {
    "200": {
      "description": "Successful Response",
      "content": {
        "application/json": {
          "schema": {
            "$ref": "#/components/schemas/SubscriptionInfo"
          }
        }
      }
    },
    "400": {
      "description": "Bad Request"
    },
    "401": {
      "description": "Unauthorized"
    },
    "403": {
      "description": "Forbidden"
    },
    "404": {
      "description": "Not Found"
    }
  }
}
```

## Schemas

The endpoints above refer to these schemas with `$ref`.

### BillingPortalUrlResponse

```json
{
  "properties": {
    "billing_portal_url": {
      "type": "string",
      "format": "uri",
      "title": "Billing Portal Url",
      "description": "Stripe customer portal URL to send the user to.",
      "examples": [
        "https://billing.stripe.com/p/session/live_YWNjdF8xQTJCM0Q0RTVGNkc3SDg"
      ]
    }
  },
  "type": "object",
  "required": [
    "billing_portal_url"
  ],
  "title": "BillingPortalUrlResponse"
}
```

### Body_b48755ce

```json
{
  "properties": {
    "amount": {
      "type": "integer",
      "title": "Amount",
      "description": "Amount to buy, in the currency's minor unit (1000 = $10.00).",
      "examples": [
        1000
      ]
    },
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO currency code.",
      "examples": [
        "usd"
      ]
    },
    "return_url": {
      "type": "string",
      "title": "Return Url",
      "description": "URL Checkout returns the user to.",
      "examples": [
        "https://todoist.com/app/settings/automations"
      ]
    },
    "workspace_id": {
      "anyOf": [
        {
          "type": "integer",
          "maximum": 1.8446744073709552e+19,
          "minimum": 1.0
        },
        {
          "type": "null"
        }
      ],
      "title": "Workspace Id",
      "description": "The workspace whose wallet this is about. Omit for the caller's personal wallet. An unknown workspace, or one the caller isn't an admin or member of, is `WORKSPACE_NOT_FOUND` (404); guests count as non-members.",
      "examples": [
        123456
      ]
    }
  },
  "type": "object",
  "required": [
    "amount",
    "currency",
    "return_url"
  ],
  "title": "Body"
}
```

### Body_e4b32da9

```json
{
  "properties": {
    "reason_flag": {
      "type": "string",
      "title": "Reason Flag",
      "description": "Short identifier of the cancellation reason category.",
      "default": "",
      "examples": [
        "too_expensive"
      ]
    },
    "reason_description": {
      "type": "string",
      "title": "Reason Description",
      "description": "Human-readable label of the cancellation reason.",
      "default": "",
      "examples": [
        "It's too expensive for me"
      ]
    },
    "reason_text": {
      "type": "string",
      "title": "Reason Text",
      "description": "Free-form feedback from the user about the cancellation.",
      "default": "",
      "examples": [
        "I no longer use it daily."
      ]
    },
    "return_url": {
      "type": "string",
      "title": "Return Url",
      "description": "URL the user is redirected to after completing the flow.",
      "default": "",
      "examples": [
        "https://todoist.com/app/settings/subscription"
      ]
    }
  },
  "type": "object",
  "title": "Body"
}
```

### PlanPrice

```json
{
  "properties": {
    "amount": {
      "type": "string",
      "title": "Amount",
      "description": "Formatted plan price amount.",
      "examples": [
        "600"
      ]
    },
    "raw_amount": {
      "type": "number",
      "title": "Raw Amount",
      "description": "Raw plan price amount in the currency's smallest unit.",
      "examples": [
        600.0
      ]
    },
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO 4217 currency code.",
      "examples": [
        "USD"
      ]
    },
    "billing_cycle": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "monthly",
            "yearly"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Billing Cycle",
      "description": "Billing cycle for the plan price, or `null` if unknown.",
      "examples": [
        "monthly"
      ]
    },
    "tax_behavior": {
      "type": "string",
      "enum": [
        "exclusive",
        "inclusive",
        "unspecified"
      ],
      "title": "Tax Behavior",
      "description": "Tax behavior for the plan price.",
      "examples": [
        "exclusive"
      ]
    }
  },
  "type": "object",
  "required": [
    "amount",
    "raw_amount",
    "currency",
    "billing_cycle",
    "tax_behavior"
  ],
  "title": "PlanPrice"
}
```

### SubscriptionInfo

```json
{
  "properties": {
    "status": {
      "type": "string",
      "enum": [
        "none",
        "autorenew",
        "canceled",
        "expired",
        "trial",
        "trial_canceled"
      ],
      "title": "Status",
      "description": "Current subscription status. `none` = never subscribed, `autorenew` = active and renewing, `canceled` = active until period end, `expired` = no longer active, `trial` = in free trial, `trial_canceled` = trial that will not convert.",
      "examples": [
        "autorenew"
      ]
    },
    "plan": {
      "type": "string",
      "enum": [
        "free",
        "pro",
        "pro_gifted",
        "pro_legacy"
      ],
      "title": "Plan",
      "description": "Plan tier the user is currently on.",
      "examples": [
        "pro"
      ]
    },
    "expiration_date": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Expiration Date",
      "description": "Date the current paid period ends, in `YYYY-MM-DD` format.",
      "examples": [
        "2026-12-31"
      ]
    },
    "activation_method": {
      "type": "string",
      "enum": [
        "stripe",
        "appstore",
        "playstore",
        "teams",
        "manual"
      ],
      "title": "Activation Method",
      "description": "How the active subscription was purchased.",
      "examples": [
        "stripe"
      ]
    },
    "plan_price": {
      "anyOf": [
        {
          "$ref": "#/components/schemas/PlanPrice"
        },
        {
          "type": "null"
        }
      ],
      "description": "Price of the active plan, when known.",
      "examples": [
        null
      ]
    },
    "billing_portal_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Billing Portal Url",
      "description": "One-time URL to the Stripe customer portal, if available.",
      "examples": [
        null
      ]
    },
    "billing_portal_switch_to_annual_url": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Billing Portal Switch To Annual Url",
      "description": "One-time URL to a Stripe portal flow for switching from monthly to annual billing, if eligible.",
      "examples": [
        null
      ]
    },
    "has_billing_portal": {
      "type": "boolean",
      "title": "Has Billing Portal",
      "description": "`true` when the user has a Stripe customer with a portal session.",
      "examples": [
        true
      ]
    },
    "has_billing_portal_switch_to_annual": {
      "type": "boolean",
      "title": "Has Billing Portal Switch To Annual",
      "description": "`true` when a switch-to-annual flow is available in Stripe.",
      "examples": [
        false
      ]
    },
    "invoice_credit_balance": {
      "anyOf": [
        {
          "additionalProperties": {
            "type": "integer"
          },
          "type": "object"
        },
        {
          "type": "null"
        }
      ],
      "title": "Invoice Credit Balance",
      "description": "Stripe customer credit balance, keyed by ISO 4217 currency code, with values in the smallest currency unit (e.g. cents).",
      "examples": [
        {
          "usd": 0
        }
      ]
    },
    "has_switch_legacy_to_current": {
      "type": "boolean",
      "title": "Has Switch Legacy To Current",
      "description": "`true` when the user is eligible to switch from a legacy Pro price to the current one.",
      "examples": [
        false
      ]
    }
  },
  "type": "object",
  "required": [
    "status",
    "plan",
    "expiration_date",
    "activation_method",
    "plan_price",
    "billing_portal_url",
    "billing_portal_switch_to_annual_url",
    "has_billing_portal",
    "has_billing_portal_switch_to_annual",
    "invoice_credit_balance",
    "has_switch_legacy_to_current"
  ],
  "title": "SubscriptionInfo"
}
```

### _CreditStateResponse

```json
{
  "properties": {
    "available_units": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Available Units",
      "description": "Balance net of in-flight holds (`available_cached - held`). The ledger unit, kept for support and reconciliation; display `available_amount` instead. Not a permission to start a run: use `usage_allowed` for that. `null`, like the other three balance fields, for a workspace member who isn't an admin.",
      "examples": [
        4186300
      ]
    },
    "available_amount": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Available Amount",
      "description": "`available_units` in money, in microdollars (millionths of one `currency` unit), so 8372600 is $8.3726. The scale is exact for every amount this API reports; cents and hundredths of a cent both truncate a single chat message to zero. Converted by the server because the client cannot: the ratio is the account's pinned price and only the backend holds it.",
      "examples": [
        8372600
      ]
    },
    "settled_units": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Settled Units",
      "description": "Balance after settled credits and debits, before in-flight holds (`available_cached`).",
      "examples": [
        4186300
      ]
    },
    "settled_amount": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Settled Amount",
      "description": "`settled_units` in money, in microdollars (millionths of one `currency` unit), using the account's pinned price.",
      "examples": [
        8372600
      ]
    },
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO currency code every amount in this response is in. Fixed for the life of the account: the first top-up pins the price, and the price fixes the currency. A user who has not topped up yet reads the currency they will be charged in: their Pro subscription's, or USD when they have none.",
      "examples": [
        "usd"
      ]
    },
    "usage_allowed": {
      "type": "boolean",
      "title": "Usage Allowed",
      "description": "Whether a new run may start right now.",
      "examples": [
        true
      ]
    },
    "low_balance": {
      "type": "boolean",
      "title": "Low Balance",
      "description": "Balance is at or below the low-balance threshold.",
      "examples": [
        false
      ]
    },
    "is_settled_low_balance": {
      "type": "boolean",
      "title": "Is Settled Low Balance",
      "description": "Whether the settled balance before in-flight holds is at or below the low-balance threshold.",
      "examples": [
        true,
        false
      ]
    },
    "usage_block_reason": {
      "anyOf": [
        {
          "type": "string",
          "enum": [
            "temporarily_unavailable",
            "insufficient_credit",
            "plan_required",
            "billing_paused"
          ]
        },
        {
          "type": "null"
        }
      ],
      "title": "Usage Block Reason",
      "description": "Why a new run cannot start, or `null` when it can. `temporarily_unavailable` means existing holds are the only blocker; `insufficient_credit` means the settled balance is at or below the configured floor; `plan_required` means the user has no Pro, or the workspace no paid Business plan, and the credit waits until it does; `billing_paused` means the compute subscription was cancelled, and the credit waits for the next top-up.",
      "examples": [
        null,
        "temporarily_unavailable",
        "insufficient_credit"
      ]
    },
    "depleted": {
      "type": "boolean",
      "title": "Depleted",
      "description": "No credit is left for another run after in-flight holds. Equivalent to `usage_allowed` being false, except when `usage_block_reason` is `plan_required` or `billing_paused`: those block runs while credit is still there.",
      "examples": [
        false
      ]
    },
    "min_top_up_amount": {
      "type": "integer",
      "title": "Min Top Up Amount",
      "description": "The smallest top-up `top_up` accepts, in `currency`'s minor unit (500 is $5.00; yen have no minor unit, so 700 is 700 yen).",
      "examples": [
        500
      ]
    },
    "top_up_presets": {
      "items": {
        "type": "integer"
      },
      "type": "array",
      "title": "Top Up Presets",
      "description": "Top-up amounts to offer, in `currency`'s minor unit, smallest first. Each is at least `min_top_up_amount`.",
      "examples": [
        [
          500,
          1000,
          2000
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "available_units",
    "available_amount",
    "settled_units",
    "settled_amount",
    "currency",
    "usage_allowed",
    "low_balance",
    "is_settled_low_balance",
    "usage_block_reason",
    "depleted",
    "min_top_up_amount",
    "top_up_presets"
  ],
  "title": "_CreditStateResponse"
}
```

### _HistoryEntry

```json
{
  "properties": {
    "id": {
      "type": "integer",
      "title": "Id",
      "description": "Ledger entry id.",
      "examples": [
        8814
      ]
    },
    "type": {
      "type": "string",
      "title": "Type",
      "description": "What moved the balance: `usage_settled`, `top_up`, `adjustment`, `void`, `expiry` or `chargeback`.",
      "examples": [
        "usage_settled"
      ]
    },
    "created_at": {
      "type": "string",
      "title": "Created At",
      "description": "When the entry was written, UTC.",
      "examples": [
        "2026-06-10T14:00:00Z"
      ]
    },
    "category": {
      "type": "string",
      "title": "Category",
      "description": "What produced the entry, within its `type`: `automation_run`, `paid`, `promotional` or `stripe`.",
      "examples": [
        "automation_run"
      ]
    },
    "status": {
      "type": "string",
      "title": "Status",
      "description": "Outcome of the run this entry settles: `success`, `failed`, `cancelled`, `timed_out` or `partial`. Present on `usage_settled` only. It does not change the amount: every terminal outcome draws the cost the run actually incurred.",
      "examples": [
        "success"
      ]
    },
    "charged_units": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Charged Units",
      "description": "Units taken off the balance, as a positive number, or `null` on a credit, which has no debit side. `0` is a real value and not an absent one: a run that failed before it reached the provider cost nothing. Not a usage figure on its own: `void`, `expiry` and `chargeback` are debits too, so summing this field across types overstates usage by every reversed grant. Filter on `type` to total usage.",
      "examples": [
        13700
      ]
    },
    "charged_amount": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Charged Amount",
      "description": "`charged_units` in money, in microdollars (millionths of one page-level `currency` unit), so 27400 is $0.0274. `null` exactly when `charged_units` is.",
      "examples": [
        27400
      ]
    },
    "credited_units": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Credited Units",
      "description": "Units added to the balance, as a positive number, or `null` on a debit. Set on credits (`top_up`, `adjustment`).",
      "examples": [
        5000000
      ]
    },
    "credited_amount": {
      "anyOf": [
        {
          "type": "integer"
        },
        {
          "type": "null"
        }
      ],
      "title": "Credited Amount",
      "description": "`credited_units` in money, in microdollars (millionths of one page-level `currency` unit), so 10000000 is $10.00. `null` exactly when `credited_units` is.",
      "examples": [
        10000000
      ]
    },
    "refs": {
      "additionalProperties": true,
      "type": "object",
      "title": "Refs",
      "description": "Free-form attribution for the entry. Contents vary by `type` and are not part of the API contract."
    },
    "description": {
      "type": "string",
      "title": "Description",
      "description": "Human-readable summary of the entry.",
      "examples": [
        "Top-up USD 1000"
      ]
    },
    "initiating_user_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Initiating User Id",
      "description": "On a workspace's wallet, the user ID of the member behind the entry: who the usage was for, or the admin who topped up. `null` on a personal wallet, and for entries with no person behind them, such as `expiry`, or usage recorded before the member was sent.",
      "examples": [
        "12345",
        null
      ]
    }
  },
  "type": "object",
  "required": [
    "id",
    "type",
    "created_at",
    "charged_units",
    "charged_amount",
    "credited_units",
    "credited_amount",
    "initiating_user_id"
  ],
  "title": "_HistoryEntry"
}
```

### _HistoryResponse

```json
{
  "properties": {
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO currency code every amount in this page is in. On the page rather than on each entry: the account pins one price for its whole life (spec section 8), so no two entries can differ.",
      "examples": [
        "usd"
      ]
    },
    "entries": {
      "items": {
        "$ref": "#/components/schemas/_HistoryEntry"
      },
      "type": "array",
      "title": "Entries",
      "description": "Ledger entries in the current page, newest first."
    },
    "has_more": {
      "type": "boolean",
      "title": "Has More",
      "description": "Whether another page of results is available.",
      "examples": [
        false
      ]
    },
    "next_cursor": {
      "type": "string",
      "title": "Next Cursor",
      "description": "Cursor for the next page of results.",
      "examples": [
        "14540000435w8hj8pXXwPQJJch.X9DBH8ya2Xenok55"
      ]
    }
  },
  "type": "object",
  "required": [
    "currency",
    "entries",
    "has_more"
  ],
  "title": "_HistoryResponse"
}
```

### _TopUpResponse

```json
{
  "properties": {
    "session_id": {
      "type": "string",
      "title": "Session Id",
      "description": "The created Stripe Checkout session id.",
      "examples": [
        "cs_live_a1b2c3"
      ]
    },
    "checkout_url": {
      "type": "string",
      "title": "Checkout Url",
      "description": "Hosted Stripe Checkout URL to redirect the user to for payment.",
      "examples": [
        "https://checkout.stripe.com/c/pay/cs_live_a1b2c3"
      ]
    }
  },
  "type": "object",
  "required": [
    "session_id",
    "checkout_url"
  ],
  "title": "_TopUpResponse"
}
```

### _UsageGroupResponse

```json
{
  "properties": {
    "automation_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Automation Id",
      "description": "With `group_by=automation`, the automation this usage belongs to. Null for usage recorded before runs carried their automation, such as the runs of an automation deleted since, and for test runs of a draft that was never published. Those group by `description` instead. Always null with `group_by=user`.",
      "examples": [
        "0199a3c4-5b6e-7f80-9a1b-2c3d4e5f6a7b"
      ]
    },
    "user_id": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "User Id",
      "description": "With `group_by=user`, the user ID of the member this usage was for. Null for usage recorded without a member, which groups by `description` instead. Always null with `group_by=automation`.",
      "examples": [
        "12345"
      ]
    },
    "description": {
      "anyOf": [
        {
          "type": "string"
        },
        {
          "type": "null"
        }
      ],
      "title": "Description",
      "description": "The label of the newest usage in the group, so a renamed automation shows its current name. Prefer the client's own name for a known `automation_id`.",
      "examples": [
        "Automation: Daily digest"
      ]
    },
    "usage_count": {
      "type": "integer",
      "title": "Usage Count",
      "description": "How many usage events settled in the group.",
      "examples": [
        42
      ]
    },
    "charged_units": {
      "type": "integer",
      "title": "Charged Units",
      "description": "Units the group took off the balance, as a positive number. Display `charged_amount` instead.",
      "examples": [
        574000
      ]
    },
    "charged_amount": {
      "type": "integer",
      "title": "Charged Amount",
      "description": "`charged_units` in money, in microdollars.",
      "examples": [
        1148000
      ]
    }
  },
  "type": "object",
  "required": [
    "automation_id",
    "user_id",
    "description",
    "usage_count",
    "charged_units",
    "charged_amount"
  ],
  "title": "_UsageGroupResponse"
}
```

### _UsageSummaryResponse

```json
{
  "properties": {
    "currency": {
      "type": "string",
      "title": "Currency",
      "description": "ISO currency code every amount in this response is in. Fixed for the life of the account: the first top-up pins the price, and the price fixes the currency. A user who has not topped up yet reads the currency they will be charged in.",
      "examples": [
        "usd"
      ]
    },
    "usage_count": {
      "type": "integer",
      "title": "Usage Count",
      "description": "How many usage events settled in the window. One per unit of billable work: an automation run, or a chat message. Narrow it with `category` to count one kind. A run that failed before it reached the provider counts here and adds 0 to the amounts: it happened, and it cost nothing.",
      "examples": [
        412
      ]
    },
    "charged_units": {
      "type": "integer",
      "title": "Charged Units",
      "description": "Units those events took off the balance, as a positive number. The ledger unit, kept for support and reconciliation; display `charged_amount` instead. Covers settled usage only, so it is smaller than the sum of `charged_units` over the same window in `/history`, which also carries reversed grants.",
      "examples": [
        5643900
      ]
    },
    "charged_amount": {
      "type": "integer",
      "title": "Charged Amount",
      "description": "`charged_units` in money, in microdollars (millionths of one `currency` unit), so 11287800 is $11.2878. Divide by `usage_count` for the per-run figure; guard the zero case, because an account with no usage reports 0 for both.",
      "examples": [
        11287800
      ]
    },
    "groups": {
      "items": {
        "$ref": "#/components/schemas/_UsageGroupResponse"
      },
      "type": "array",
      "title": "Groups",
      "description": "The same total split by `group_by`, largest spend first. Present only when `group_by` is set. The counts and units add up to the totals above; each amount rounds on its own.",
      "examples": [
        [
          {
            "automation_id": "0199a3c4-5b6e-7f80-9a1b-2c3d4e5f6a7b",
            "charged_amount": 1148000,
            "charged_units": 574000,
            "description": "Automation: Daily digest",
            "usage_count": 42
          },
          {
            "charged_amount": 82200,
            "charged_units": 41100,
            "description": "Automation: Triage",
            "usage_count": 3
          }
        ]
      ]
    }
  },
  "type": "object",
  "required": [
    "currency",
    "usage_count",
    "charged_units",
    "charged_amount"
  ],
  "title": "_UsageSummaryResponse"
}
```
