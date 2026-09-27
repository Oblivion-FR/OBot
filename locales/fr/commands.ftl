## Commandes slash : leur description dans Discord et leurs réponses.
## Les identifiants sont les mêmes dans chaque langue, voir locales/en/commands.ftl.

command-healthcheck-description = Vérifie que le bot fonctionne
command-version-description = Version d'OBot en service, avec un lien vers son code source
command-verify-description = Obtiens tes rôles en prouvant que tu possèdes un compte Minecraft
command-verify-username = pseudo
command-whois-description = Le compte Minecraft avec lequel un membre s'est vérifié, ou le membre qui a vérifié un compte
command-whois-member = membre
command-whois-member-description = Un membre de ce serveur
command-whois-minecraft = minecraft
command-whois-minecraft-description = Un pseudo Minecraft
command-verify-username-description = Ton pseudo Minecraft

healthcheck-reply = Salut !
version-reply =
    OBot v{ $version } ({ $commit })
    Code source : <{ $repository }>

verify-not-set-up = La vérification n'est pas encore configurée sur ce serveur, demande à un admin de la configurer dans le panel.
verify-unknown-account = Aucun compte Minecraft ne s'appelle `{ $name }`.
verify-never-joined = `{ $name }` ne s'est jamais connecté à Hypixel.
verify-no-discord-linked = `{ $name }` n'a pas de Discord lié sur Hypixel. En jeu, ouvre ton profil → Social Media → Discord et entre `{ $discord }`, puis réessaie.
verify-linked-to-someone-else = `{ $name }` est lié au Discord `{ $linked }`, pas à toi (`{ $discord }`).
verify-already-verified = Tu es déjà vérifié en tant que `{ $name }`. Demande à un admin de retirer ta vérification pour lier un autre compte.
verify-account-taken = `{ $name }` est déjà lié à un autre membre de ce serveur ({ $member }).
verify-done = Tu es vérifié en tant que `{ $name }` !
verify-roles-added = Rôles ajoutés : { $roles }
verify-roles-removed = Rôles retirés : { $roles }
verify-nickname-set = Pseudo changé en `{ $nickname }`
verify-nickname-skipped = ⚠️ Pseudo non changé en `{ $nickname }` : { $reason }.

whois-missing = Donne un membre, un pseudo Minecraft, ou les deux.
whois-verified = { $member } est vérifié en tant que [`{ $name }`](<https://namemc.com/profile/{ $uuid }>) depuis le { $since }.
whois-verified-by = { $member } a été vérifié en tant que [`{ $name }`](<https://namemc.com/profile/{ $uuid }>) par { $admin } le { $since }.
whois-not-verified = { $member } n'est pas vérifié.
whois-unknown-account = Aucun compte Minecraft ne s'appelle `{ $name }`.
whois-account-free = `{ $name }` n'est lié à aucun membre de ce serveur.

## Pourquoi un membre n'a pas pu être renommé, dans les réponses de /verify et le panel

rename-no-server = le bot n'a pas pu charger ce serveur
rename-owner = les bots ne peuvent pas renommer le propriétaire du serveur
rename-role-too-high = son rôle le plus haut est au niveau de celui du bot ou au-dessus
rename-missing-permission = il manque peut-être au bot la permission Gérer les pseudos
