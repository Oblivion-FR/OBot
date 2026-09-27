# Using the panel

## Access

Log in with Discord. A server appears in the panel when the bot is in it and you have
**Administrator**, or both **Manage Server** and **Manage Roles**, there. This is checked again
on every page, so losing the permission on Discord also removes panel access.

Like on Discord itself, you can only pick roles below your own highest role (the server owner has
no limit). Otherwise the verified role could be used to give yourself a role you can't assign.

## Language

The panel is in English and French. It follows your Discord language at login, or your browser's;
the **EN / FR** buttons in the top bar (or on the login page) switch it and remember the choice
on that browser. Slash command replies follow the language of each member's Discord client.

## How verification works

A member runs `/verify <minecraft name>`. OBot checks that the Discord account linked in that
player's Hypixel profile (in game: profile → Social Media → Discord) is the member's own, which
proves they own the Minecraft account. Then it:

1. gives the verified role and removes the unverified role,
2. gives or removes every role used in a rule, depending on whether the rule matches,
3. renames the member, if nicknames are turned on,
4. remembers which Minecraft account the member verified with.

Running `/verify` again refreshes roles and nickname. OBot also refreshes every verified member
on its own every 3 hours (set by `RESYNC_INTERVAL_HOURS`), with current Hypixel data and
Minecraft name, like the panel's **Re-verify** does. A refresh keeps the date of the
verification. Members who left the server are skipped, and keep their link if they come back.

Each member can be linked to one Minecraft account, and each account to one member, per server.

A failed check is always retried live against Hypixel, so a member who links their Discord right
after a failed attempt can verify straight away. Successful lookups are reused for up to
10 minutes, so a rank change can take that long to show on a new `/verify`.

## Sections

### Overview

A summary of the server's setup, with links to each section.

### Verification

- **Verification roles**: the role given by `/verify`, and optionally a role it removes. With no
  verified role, `/verify` is disabled.
- **Hypixel guild**: the guild used by guild rules, guild rank tags and the guild columns. Type
  its name; the panel stores its ID, so a rename doesn't break the link.
- **Members**: every member of the server with their verification status.
  - Search by Discord or Minecraft name.
  - Click a column header to sort: ascending, then descending, then unsorted.
  - **Columns** chooses which columns are shown. The choice is saved in your browser; the guild
    XP column starts hidden.
  - **Re-verify** refreshes a verified member's roles and nickname from their stored account.
  - **Verify…** verifies a member on their behalf, with the same Hypixel check as `/verify`.
  - **Remove** removes a verification: the verified and rule roles, the stored account, and the
    nickname. Like on Discord, you can only do it to members below your highest role.

Statuses: **Verified**, **Verified by admin** (done from the panel), **Role only** (has the
verified role but no stored account, for example verified before accounts were stored), and
**Not verified**.

### Role rules

A rule gives a role when a condition matches, and removes it when it stops matching:

| Condition                   | Matches when                                                     |
|-----------------------------|------------------------------------------------------------------|
| Hypixel rank is …           | The player has exactly that rank (VIP to MVP++, YOUTUBER, STAFF) |
| Hypixel rank is No rank     | The player has no paid or special rank                           |
| Member of the Hypixel guild | The player is in the linked guild                                |
| Guild rank is …             | The player has that rank in the linked guild                     |

Guild ranks are picked from the linked guild's current ranks.

#### Groups and separator roles

Separator roles are the dividers of a member's role list, like `━━ Ranks ━━` placed above the
rank roles. A group ties a separator to rules: members get the separator while at least one rule
of the group matches, and lose it when none does. A member without any rank role then has no
empty `━━ Ranks ━━` header.

Create a group with its name and separator role, then pick the group when adding a rule. Rules
can also stay without a group. Deleting a group keeps its rules, without a group. Removing a
member's verification also removes their separators.

### Nickname

Builds members' nicknames from their Hypixel data, for example `[MVP+] Notch [OFC]`.

- Each field (Hypixel rank, Minecraft name, guild rank tag, guild tag) can be shown or hidden,
  ordered, and wrapped in a prefix and suffix.
- A field without a value, like a player without a rank, is skipped with its prefix and suffix.
- **Texts per rank** changes the prefix, label or suffix of a single Hypixel rank or guild rank,
  like `★MVP++★` or a crown instead of `[GM]`. The inputs show the current texts: change only
  the parts you want, the others keep following the field's defaults. An emptied part shows
  nothing, and "No rank" can get a label so players without a rank show one too.
- Discord nicknames are limited to 32 characters: above that, the field with the highest
  importance number is dropped first.
- The preview updates as you edit.

Discord never lets bots rename the server owner, or members whose highest role is at or above the
bot's: their roles are still updated, and the reply says the rename was skipped.
