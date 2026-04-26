# sunburn.json Specification

## Overview

`sunburn.json` is an optional JSON file placed at the root of the FAT32 boot
partition of a disk image.  When sun-burn opens an image it looks for this
file.  If found, the app reads the manifest and presents a guided setup wizard
to the user before flashing.  If the file is absent the image is flashed in
**basic mode** — no wizard, no configuration writes.

---

## Location

```
<boot partition root>/sunburn.json
```

The boot partition is the first partition in the MBR partition table and must
be formatted as FAT32.

---

## Top-level schema

| Field         | Type             | Required | Description                          |
|---------------|------------------|----------|--------------------------------------|
| `version`     | `string`         | yes      | Must be `"1"`.                       |
| `name`        | `string`         | yes      | Human-readable image name.           |
| `description` | `string`         | no       | Short description shown in the UI.   |
| `steps`       | `array<Step>`    | yes      | Ordered list of wizard steps.        |

---

## Step schema

| Field    | Type              | Required | Description                                  |
|----------|-------------------|----------|----------------------------------------------|
| `id`     | `string`          | yes      | Unique identifier (used internally).         |
| `title`  | `string`          | yes      | Heading shown at the top of the step.        |
| `fields` | `array<Field>`    | yes      | Input fields to collect on this step.        |
| `writes` | `array<WriteRule>`| yes      | Files to write to the boot partition.        |

---

## Field schema

| Field       | Type                  | Required | Description                                           |
|-------------|-----------------------|----------|-------------------------------------------------------|
| `id`        | `string`              | yes      | Unique identifier; used in template substitution.     |
| `label`     | `string`              | yes      | Human-readable label shown next to the input.         |
| `type`      | `FieldType`           | yes      | Input widget type (see below).                        |
| `required`  | `boolean`             | no       | Defaults to `false`.                                  |
| `default`   | `string`              | no       | Pre-filled value; used when the field is not filled.  |
| `show_when` | `ShowWhen`            | no       | Conditional visibility rule (see below).              |
| `options`   | `array<SelectOption>` | no       | Required when `type` is `"select"`.                   |

### Field types

| Value              | Widget                                             |
|--------------------|----------------------------------------------------|
| `text`             | Plain single-line text input.                      |
| `password`         | Masked text input.                                 |
| `wifi-picker`      | Native or scanned Wi-Fi SSID chooser.              |
| `ssh-key-picker`   | File-picker / paste for an SSH public key.         |
| `country-picker`   | Dropdown seeded from the ISO 3166-1 country list.  |
| `timezone-picker`  | Dropdown seeded from IANA timezone database.       |
| `toggle`           | Boolean on/off switch; value is `"true"`/`"false"`.|
| `select`           | Dropdown from a static list; requires `options`.   |

### SelectOption schema (for `select` fields)

| Field   | Type     | Description              |
|---------|----------|--------------------------|
| `value` | `string` | Value stored/substituted.|
| `label` | `string` | Human-readable label.    |

---

## ShowWhen (conditional visibility)

When a field has `show_when`, it is hidden until the referenced field equals
the specified value.

```json
"show_when": { "field": "ssh_enabled", "value": "true" }
```

| Field   | Type     | Description                                           |
|---------|----------|-------------------------------------------------------|
| `field` | `string` | `id` of the field to observe.                         |
| `value` | `string` | The value that makes the dependent field visible.     |

If a hidden field has no `default` and is not filled, its placeholder is
substituted with an empty string.

---

## WriteRule and template substitution

After the user completes a step the app writes one or more files to the boot
partition using the `writes` array.

| Field      | Type     | Description                                  |
|------------|----------|----------------------------------------------|
| `path`     | `string` | Relative path from the boot partition root.  |
| `template` | `string` | Content with `{{field_id}}` placeholders.    |

**Substitution rules:**

1. Every `{{field_id}}` token is replaced with the collected value for that
   field.
2. If a field was not shown (due to `show_when`) and has no default, the
   placeholder is replaced with an empty string.
3. Unknown placeholders (no matching field id) are left as-is.
4. Substitution is performed once per write rule, after the step is completed.
5. The written content is UTF-8 encoded.

---

## Complete example manifest

```json
{
  "version": "1",
  "name": "Solar Monitor",
  "description": "Home solar monitoring stack",
  "steps": [
    {
      "id": "network",
      "title": "Network Setup",
      "fields": [
        { "id": "ssid",     "type": "wifi-picker",    "label": "WiFi Network", "required": true },
        { "id": "password", "type": "password",        "label": "Password",     "required": true },
        { "id": "country",  "type": "country-picker",  "label": "Country",      "required": false, "default": "US" }
      ],
      "writes": [
        { "path": "wifi.txt", "template": "ssid={{ssid}}\npassword={{password}}\ncountry={{country}}" }
      ]
    },
    {
      "id": "device",
      "title": "Device",
      "fields": [
        { "id": "hostname", "type": "text",            "label": "Device name", "required": true,  "default": "my-device" },
        { "id": "timezone", "type": "timezone-picker", "label": "Timezone",    "required": false, "default": "auto" }
      ],
      "writes": [
        { "path": "device.txt", "template": "hostname={{hostname}}\ntimezone={{timezone}}" }
      ]
    },
    {
      "id": "access",
      "title": "Access",
      "fields": [
        { "id": "ssh_enabled", "type": "toggle",       "label": "Enable SSH",        "required": false, "default": "false" },
        { "id": "ssh_key",     "type": "ssh-key-picker","label": "SSH Public Key",   "required": false,
          "show_when": { "field": "ssh_enabled", "value": "true" } }
      ],
      "writes": [
        { "path": "access.txt", "template": "ssh_enabled={{ssh_enabled}}\nssh_key={{ssh_key}}" }
      ]
    }
  ]
}
```

---

## Basic mode

If the boot partition does not contain `sunburn.json`, sun-burn treats the
image as a basic image.  No wizard is shown; the image is written to the
selected drive as-is without any configuration step.
