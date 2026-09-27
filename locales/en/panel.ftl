## Web panel: pages, forms and messages.
## Message ids are shared by every language, see locales/fr/panel.ftl.

## Frame: login, server bar, top bar, footer

login-title = Log in
login-heading = OBot dashboard
login-intro = Configure verification, role rules and nicknames for your servers. You need Administrator, or Manage Server and Manage Roles.
login-button = Log in with Discord
rail-servers = Servers
rail-home = Home
rail-add = Add OBot to a server
log-out = Log out
language = Language
footer-source = Source
unknown-commit = unknown

## Home

home-title = Servers
home-welcome = Welcome back, { $name }
home-intro = Pick a server to configure. Only servers where OBot is and you can manage roles are listed.
home-configure = Configure →
home-add = Add OBot to a server
home-add-hint = Opens Discord's invite screen

## Server sidebar

server-settings = Server settings
nav-sections = Sections
nav-general = General
nav-verification-group = Verification
nav-overview = Overview
nav-verification = Verification
nav-rules = Role rules
nav-nickname = Nickname

## Errors shown at the top of a page

error-guild-not-found = No Hypixel guild has this name.
error-no-hypixel-guild = Link a Hypixel guild before adding guild rules.
error-missing-value = This rule needs a value.
error-unknown-guild-rank = This rank doesn't exist in the Hypixel guild anymore.
error-guild-ranks-unavailable = Couldn't load the guild ranks from Hypixel, try again later.
error-role-not-allowed = You can only pick roles below your highest role.
error-missing-group-name = A group needs a name.
error-unknown-group = This group doesn't exist anymore.

## Overview

overview-intro = How verification is set up on { $server }.
badge-active = Active
badge-not-set-up = Not set up
badge-linked = Linked
badge-not-linked = Not linked
badge-none = None
badge-on = On
badge-off = Off
badge-rules-active = { $count } active
overview-verification = Verification
overview-verified-role-hint = Given on /verify
overview-no-verified-role = Pick a verified role to enable /verify.
overview-hypixel-guild = Hypixel guild
overview-no-guild = Link one to use guild rules and tags.
overview-rules = Role rules
overview-rule-count =
    { $count ->
        [one] { $count } rule
       *[other] { $count } rules
    }
overview-rules-hint = Roles given from Hypixel rank and guild.
overview-nickname = Nickname
overview-nickname-hint = Example of a renamed member.
overview-configure = Configure →
overview-manage = Manage →
overview-how-title = How members verify
overview-how-hint = What to tell your members.
overview-how-link = On Hypixel, open the profile menu → Social Media → Discord, and enter their Discord username.
overview-how-verify = In this server, run /verify with their Minecraft name.
overview-how-roles = OBot checks the link, then updates their roles.
overview-how-roles-nickname = OBot checks the link, then updates their roles and nickname.

## Verification page

verification-intro = What /verify gives and removes, and which Hypixel guild counts for guild rules.
verification-roles = Verification roles
verification-roles-hint = Greyed out roles are above the bot's or your highest role.
verified-role = Verified role (given)
verified-role-none = None: /verify disabled
unverified-role = Unverified role (removed)
role-none = None
save-roles = Save roles
hypixel-guild = Hypixel guild
hypixel-guild-hint = Needed for guild member and guild rank rules, and for guild tags in nicknames. Leave empty to unlink.
guild-name = Guild name
guild-not-linked = Not linked
save-guild = Save guild

## Member table

members = Members
members-count = { $verified } of { $total } verified
members-matching = { $count } matching
members-unavailable = Couldn't list the server members. Enable Server Members Intent in the Discord developer portal (Bot → Privileged Gateway Intents), then reload.
members-guild-not-linked = Link a Hypixel guild to fill the guild columns.
members-guild-unavailable = Couldn't load the guild from Hypixel, the guild columns are empty.
search = Search
search-placeholder = Discord or Minecraft name, then Enter
columns = Columns
visible-columns = Visible columns
column-name = Member
column-minecraft = Minecraft
column-status = Status
column-joined = Joined server
column-verified = Verified on
column-guild-joined = Joined guild
column-xp = Guild XP (7 days)
column-actions = Actions
no-member-matches = No member matches.
member-pages = Member pages
page-previous = ← Previous
page-next = Next →
page-of = Page { $page } of { $pages }
status-verified = Verified
status-verified-by-admin = Verified by admin
status-role-only = Role only
status-role-only-hint = Has the verified role but no linked Minecraft account, e.g. verified before accounts were stored
status-not-verified = Not verified
pick-verified-role-first = Pick a verified role first
reverify = Re-verify
verify-member = Verify…
remove = Remove
remove-confirm = Remove { $name }'s verification? Their verification and rule roles are removed and their nickname is reset.
verify-dialog-hint = Same check as /verify: the Minecraft account must have this member's Discord linked on Hypixel. Roles and nickname are then applied for them.
minecraft-username = Minecraft username
cancel = Cancel
verify = Verify

## Member actions: results and refusals

notice-reverified = Re-verified as { $name }.
notice-verified-by-you = Verified as { $name } by you.
notice-removed = Verification removed.
notice-added = Added { $roles }.
notice-removed-roles = Removed { $roles }.
notice-nickname-set = Nickname set to { $nickname }.
notice-nickname-reset = Nickname reset.
notice-nickname-skipped = Nickname not changed to { $nickname }: { $reason }.
notice-up-to-date = Everything was already up to date.
notice-no-roles-left = They had no verification roles left.
deleted-role = deleted role
no-linked-account = This member has no linked account, use Verify instead.
account-gone = The Minecraft account { $name } no longer exists.
cannot-manage-server = You can't manage this server anymore.
member-left = This member left the server.
bots-cannot-verify = Bots can't be verified.
enter-username = Enter a Minecraft username.
unknown-account = No Minecraft account is named { $name }.
never-joined = { $name } has never joined Hypixel.
no-discord-linked = { $name } has no Discord linked on Hypixel. Ask { $member } to link @{ $member } in the game's Social Media menu first.
linked-to-someone-else = { $name } is linked to the Discord { $linked } on Hypixel, not to @{ $member }.
member-already-verified = @{ $member } is already verified as { $name }. Remove their verification first.
account-taken = { $name } is already linked to another member ({ $member }).
server-unavailable = The bot couldn't load this server.
owner-only = Only the server owner can remove their own verification.
member-above-you = This member's highest role is at or above yours.

## Browser messages

error-session-expired = Your session expired, reload the page to log in again.
error-generic = Something went wrong, check the bot logs.
error-unreachable = Couldn't reach the panel, try again.

## Role rules page

rules-intro = Checked on each /verify: a rule's role is given when it matches and removed when it doesn't. A group's separator role is given while any of its rules matches.
rules = Rules
rules-summary =
    { $count ->
        [one] { $count } rule
       *[other] { $count } rules
    }
rules-in-groups =
    { $groups ->
        [one] in { $groups } group
       *[other] in { $groups } groups
    }
rules-empty = No rules yet. Add one below.
group-separator = separator
delete-group = Delete group
delete-group-confirm = Delete the group { $name }? Its rules are kept, without a group.
group-empty = No rules in this group yet: its separator is never given.
no-group = No group
delete-rule = Delete rule
add-rule = Add a rule
add-rule-no-guild = Link a Hypixel guild in Verification to use guild conditions.
add-rule-guild = Guild conditions use { $guild }.
rule-when = When
rule-kind-hypixel-rank = Hypixel rank is
rule-kind-guild-member = Member of the Hypixel guild
rule-kind-guild-rank = Guild rank is
hypixel-rank = Hypixel rank
guild-rank = Guild rank
guild-ranks-unavailable = Couldn't load ranks from Hypixel
guild-ranks-link-first = Link a Hypixel guild first
guild-ranks-none = No ranks found
give-role = Give role
group = Group
add-rule-button = Add rule
add-group = Add a group
add-group-hint = Groups rules under a separator role, like ━━ Ranks ━━ placed above the rank roles. Members get the separator while any rule of the group matches.
group-name = Name
group-name-placeholder = e.g. Ranks
separator-role = Separator role
add-group-button = Add group
condition-no-rank = No Hypixel rank
condition-hypixel-rank = Hypixel rank is { $rank }
condition-guild-member = Member of the Hypixel guild
condition-guild-rank = Guild rank is { $rank }
rank-no-rank = No rank

## Nickname page

nickname-intro = Rename members on /verify from their Hypixel profile.
nickname-enabled = Rename members on /verify
separator = Separator between fields
fields = Fields
fields-hint = Fields without a value (no rank, not in the guild) are skipped with their prefix and suffix. Above 32 characters, the highest importance number is dropped first.
field-show = Show
field = Field
field-order = Order
field-prefix = Prefix
field-suffix = Suffix
field-importance = Importance
field-hypixel-rank = Hypixel rank
field-ign = Minecraft name
field-guild-rank-tag = Guild rank tag
field-guild-tag = Guild tag
texts-per-rank = Texts per rank
texts-per-rank-hint = Change the prefix, label or suffix of a single rank, like ★MVP++★. Untouched parts follow the field's defaults above; an emptied part shows nothing. Ranks without a label are skipped.
texts-hypixel-ranks = Hypixel ranks
texts-guild-ranks = Guild ranks
texts-link-guild = Link a Hypixel guild in Verification to customize its ranks.
texts-guild-gone = The linked Hypixel guild doesn't exist anymore.
texts-guild-unavailable = Couldn't load the guild's ranks from Hypixel, try again later.
rank = Rank
label = Label
label-none = none
custom = Custom
preview = Preview
preview-hint = Updates as you edit. "Longest" has every field at its maximum length.
preview-example = Example
preview-longest = Longest
preview-unchanged = (unchanged)
save-nickname = Save nickname
