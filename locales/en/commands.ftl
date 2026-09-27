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
# Small print under every verification reply and the verify button message
verify-data-notice = -# To give you your roles, OBot saves your Discord ID and the Minecraft account you verify, and reads its Hypixel profile. The server's admins can see them. [Privacy policy](<{ $url }>)
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

## The log channel, in the server's language. Mentions don't ping anyone.

log-channel-set = 📋 { $admin } picked this channel for OBot's log.
log-verified = ✅ { $member } verified as `{ $name }`.
log-verified-by = ✅ { $member } was verified as `{ $name }` by { $admin }.
log-reverified-by = 🔄 { $member } was re-verified as `{ $name }` by { $admin }.
log-refreshed = 🔄 { $member } was updated from `{ $name }`'s Hypixel profile.
log-removed-by = 🗑️ { $admin } removed the verification of { $member }.
log-refused = ❌ { $member } couldn't verify as `{ $name }`: { $reason }.
log-reason-never-joined = this account never joined Hypixel
log-reason-no-discord = no Discord is linked on Hypixel
log-reason-linked-elsewhere = it's linked to the Discord `{ $linked }`
log-reason-already-verified = they are already verified as `{ $current }`
log-reason-account-taken = it's already linked to `{ $owner }`
log-added = Added { $roles }
log-removed = Removed { $roles }
log-nickname-set = nickname `{ $nickname }`
log-nickname-reset = nickname reset
log-nickname-skipped = ⚠️ nickname `{ $nickname }` not set: { $reason }

## The verify button's message, in the server's language, and its form, in the member's

verify-message =
    ## Verification
    Link your Minecraft account to get your roles.
    1. On Hypixel, open your profile → Social Media → Discord, and enter your Discord username.
    2. Click **{ verify-button }** below and enter your Minecraft username.
verify-button = Verify
verify-modal-title = Verification
verify-modal-username = Minecraft username
verify-failed = Something went wrong, try again in a moment.
