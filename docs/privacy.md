# Privacy policy

*Last updated: September 27, 2026* · [Version française](fr/confidentialite.md)

This policy explains what data OBot processes, why, how long it is kept and how to exercise your
rights. It is a translation: the French version prevails.

## 1. Who is responsible

OBot is a free and open source Discord bot, released under the BSD 3-Clause license
([source code](https://github.com/Oblivion-FR/OBot)).

- **Official instance**: it is operated by **Oblivion FR**, the data controller under the
  General Data Protection Regulation (GDPR). Contact: **privacy@oblivion.ovh**.
- **Self-hosted instances**: anyone can run their own copy of OBot. They alone are then
  responsible for it, and this policy only describes what the software does. Oblivion FR has no
  access to the data of these instances.

## 2. Data processed

OBot only collects what Minecraft account verification and its web panel need. It doesn't read
message content, uses no analytics, and sells no data.

### Members who verify

When you use `/verify` or the verify button, or an admin verifies you from the panel, OBot
stores:

- your Discord ID and the server's;
- the ID (UUID) and username of the Minecraft account you proved you own;
- the date of the verification and, when an admin made it, their Discord ID.

To verify you and keep your roles up to date, OBot also reads, without storing them, your
Hypixel profile (the Discord linked to it, your Hypixel rank), your Hypixel guild and your rank
in it. These responses are kept in memory for 10 minutes at most. OBot then changes your roles
and, if the server turned it on, your nickname in the server. This is refreshed automatically
about every 3 hours.

### Admins who use the panel

Logging in to the panel goes through Discord, with only the `identify` permission: OBot receives
neither your e-mail address nor your server list. For the length of the session, it stores:

- your ID, display name, avatar link and Discord account language;
- a hash (SHA-256) of the session token, never the token itself.

To show the member table, the panel reads the server's member list from Discord (name, avatar,
roles, join date) and the Hypixel guild XP of the last 7 days. This data is kept in memory for a
few minutes at most and isn't stored.

### Servers

For each server, OBot stores its configuration: chosen roles, linked Hypixel guild, role rules,
nickname format, server language and log channel.

### Log channel

When a server picks a log channel, OBot posts there verifications, refused verifications, panel
actions and automatic refreshes, with the members concerned. These messages are stored by
Discord: they are visible to whoever can see the channel, and deleting them is up to the
server's admins.

### Technical logs

When an error occurs, the machine hosting OBot may record Discord IDs and Minecraft usernames
in its logs, only to diagnose it.

## 3. Purposes and legal bases

| Purpose                                        | Legal basis (GDPR article 6)                                |
|------------------------------------------------|-------------------------------------------------------------|
| Verifying a Minecraft account and giving roles | Performance of the service you ask for by verifying         |
| Refreshing roles and nicknames regularly       | The server's legitimate interest in accurate roles          |
| Letting admins manage their server             | Performance of the service                                  |
| Posting in the log channel                     | The server's legitimate interest in following verifications |
| Security, abuse prevention, diagnostics        | The operator's legitimate interest                          |

## 4. Cookies

The panel only uses cookies needed for it to work or to remember your preferences, with no
tracking or advertising. They are therefore exempt from consent.

| Cookie                | Purpose                              | Lifetime                        |
|-----------------------|--------------------------------------|---------------------------------|
| `obot_session`        | Keeping you logged in                | Browser session, 7 days at most |
| `obot_oauth_state`    | Protecting the Discord login         | The length of the login         |
| `obot_lang`           | Remembering the chosen language      | 1 year                          |
| `obot_hidden_columns` | Remembering the hidden table columns | 1 year                          |

## 5. Recipients and third-party services

Your data is only shared with the services OBot needs to work:

- **Discord**: panel login, reading members, giving roles and nicknames, messages;
- **Mojang / Microsoft**: turning a Minecraft username into an ID, and back;
- **Hypixel**: reading profiles and guilds, from the Minecraft ID;
- **jsDelivr**: the panel loads the htmx library from this service, which then receives the
  visitor's IP address; avatars are loaded from Discord's network.

These services may process data outside the European Union, notably in the United States,
under their own privacy policies. A server's admins see, in the panel and the log channel, the
data of their server's members.

## 6. Retention

- **A member's verification**: until it is removed. An admin can remove it from the panel. If
  you leave the server, it is kept to give your roles back if you return, and deleted if someone
  else later verifies the same Minecraft account.
- **Panel session**: 7 days at most, or until you log out.
- **A server's configuration**: until it is deleted, including when the bot is removed from the
  server. You can ask for its deletion.
- **Data in memory**: a few minutes, and lost on every restart.

## 7. Your rights

Under the GDPR, you have the right to access, rectify, erase, restrict, object to and port your
data. To exercise them, write to **privacy@oblivion.ovh** from an address we can reply to, with
your Discord ID. You will get an answer within a month.

An admin of your server can also remove your verification right away from the panel.

If you believe your rights aren't respected, you can complain to the French data protection
authority, the CNIL ([cnil.fr](https://www.cnil.fr/fr/plaintes)), or to the authority of your
country.

## 8. Security

The panel is served over HTTPS, sessions are stored as hashes, and every action checks again,
with Discord, that you have the needed permissions in the server.

## 9. Minors

OBot is meant for Discord users, who must meet the minimum age set by Discord's terms in their
country.

## 10. Changes

This policy may change with the software. Every version is published in the
[project's repository](https://github.com/Oblivion-FR/OBot), with the history of its changes.
