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
verify-data-notice = -# Pour te donner tes rôles, OBot enregistre ton identifiant Discord et le compte Minecraft que tu vérifies, et lit son profil Hypixel. Les admins du serveur peuvent les voir. [Politique de confidentialité](<{ $url }>)
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

## Le salon de logs, dans la langue du serveur. Les mentions ne notifient personne.

log-channel-set = 📋 { $admin } a choisi ce salon pour les logs d'OBot.
log-verified = ✅ { $member } s'est vérifié en tant que `{ $name }`.
log-verified-by = ✅ { $member } a été vérifié en tant que `{ $name }` par { $admin }.
log-reverified-by = 🔄 { $member } a été revérifié en tant que `{ $name }` par { $admin }.
log-refreshed = 🔄 { $member } a été mis à jour depuis le profil Hypixel de `{ $name }`.
log-removed-by = 🗑️ { $admin } a retiré la vérification de { $member }.
log-refused = ❌ { $member } n'a pas pu se vérifier en tant que `{ $name }` : { $reason }.
log-reason-never-joined = ce compte n'a jamais rejoint Hypixel
log-reason-no-discord = aucun Discord n'est lié sur Hypixel
log-reason-linked-elsewhere = il est lié au Discord `{ $linked }`
log-reason-already-verified = il est déjà vérifié en tant que `{ $current }`
log-reason-account-taken = il est déjà lié à `{ $owner }`
log-added = Ajouté : { $roles }
log-removed = Retiré : { $roles }
log-nickname-set = pseudo `{ $nickname }`
log-nickname-reset = pseudo réinitialisé
log-nickname-skipped = ⚠️ pseudo `{ $nickname }` non appliqué : { $reason }

## Le message du bouton de vérification, dans la langue du serveur, et son formulaire, dans celle du membre

verify-message =
    ## Vérification
    Lie ton compte Minecraft pour obtenir tes rôles.
    1. Sur Hypixel, ouvre ton profil → Social Media → Discord, et entre ton nom d'utilisateur Discord.
    2. Clique sur **{ verify-button }** ci-dessous et entre ton pseudo Minecraft.
verify-button = Vérifier
verify-modal-title = Vérification
verify-modal-username = Pseudo Minecraft
verify-failed = Une erreur est survenue, réessaie dans un instant.
