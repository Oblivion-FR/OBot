## Slash commands: how they're described in Discord, and their replies.
## Message ids are shared by every language, see locales/fr/commands.ftl.

command-healthcheck-description = Check that the bot is alive
command-version-description = Which version of OBot runs, with a link to its source
command-verify-description = Get your roles by proving you own a Minecraft account
command-verify-username = username
command-verify-username-description = Your Minecraft username
command-whois-description = Which Minecraft account a member verified with, or which member verified an account
command-whois-member = member
command-whois-member-description = A member of this server
command-whois-minecraft = minecraft
command-whois-minecraft-description = A Minecraft username

healthcheck-reply = Hi!
version-reply =
    OBot v{ $version } ({ $commit })
    Source: <{ $repository }>

verify-not-set-up = Verification is not set up on this server yet, ask an admin to configure it in the panel.
verify-unknown-account = No Minecraft account is named `{ $name }`.
verify-never-joined = `{ $name }` has never joined Hypixel.
verify-no-discord-linked = `{ $name }` has no Discord linked on Hypixel. In game, open your profile → Social Media → Discord and enter `{ $discord }`, then try again.
verify-linked-to-someone-else = `{ $name }` is linked to the Discord `{ $linked }`, not to you (`{ $discord }`).
verify-already-verified = You are already verified as `{ $name }`. Ask an admin to remove your verification to link another account.
verify-account-taken = `{ $name }` is already linked to another member of this server ({ $member }).
verify-done = You are verified as `{ $name }`!
verify-roles-added = Roles added: { $roles }
verify-roles-removed = Roles removed: { $roles }
verify-nickname-set = Nickname set to `{ $nickname }`
verify-nickname-skipped = ⚠️ Nickname not changed to `{ $nickname }`: { $reason }.

whois-missing = Give a member, a Minecraft username, or both.
whois-verified = { $member } is verified as [`{ $name }`](<https://namemc.com/profile/{ $uuid }>) since { $since }.
whois-verified-by = { $member } was verified as [`{ $name }`](<https://namemc.com/profile/{ $uuid }>) by { $admin } on { $since }.
whois-not-verified = { $member } isn't verified.
whois-unknown-account = No Minecraft account is named `{ $name }`.
whois-account-free = `{ $name }` isn't linked to any member of this server.

## Why a member couldn't be renamed, in /verify replies and the panel

rename-no-server = the bot couldn't load this server
rename-owner = bots can't rename the server owner
rename-role-too-high = their highest role is at or above the bot's
rename-missing-permission = the bot may be missing the Manage Nicknames permission
