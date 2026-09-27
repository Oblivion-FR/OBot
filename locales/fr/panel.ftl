## Panel web : pages, formulaires et messages.
## Les identifiants sont les mêmes dans chaque langue, voir locales/en/panel.ftl.

## Cadre : connexion, barre des serveurs, barre du haut, pied de page

login-title = Connexion
login-heading = Tableau de bord OBot
login-intro = Configure la vérification, les règles de rôles et les pseudos de tes serveurs. Il faut être Administrateur, ou avoir Gérer le serveur et Gérer les rôles.
login-button = Se connecter avec Discord
rail-servers = Serveurs
rail-home = Accueil
rail-add = Ajouter OBot à un serveur
log-out = Se déconnecter
language = Langue
footer-source = Code source
footer-privacy = Confidentialité
footer-terms = Conditions d'utilisation
footer-privacy-path = docs/fr/confidentialite.md
footer-terms-path = docs/fr/conditions-utilisation.md
unknown-commit = inconnu

## Accueil

home-title = Serveurs
home-welcome = Bon retour, { $name }
home-intro = Choisis un serveur à configurer. Seuls les serveurs où se trouve OBot et où tu peux gérer les rôles sont listés.
home-configure = Configurer →
home-add = Ajouter OBot à un serveur
home-add-hint = Ouvre la page d'invitation de Discord

## Menu du serveur

server-settings = Paramètres du serveur
nav-sections = Sections
nav-general = Général
nav-verification-group = Vérification
nav-overview = Vue d'ensemble
nav-verification = Vérification
nav-rules = Règles de rôles
nav-nickname = Pseudo

## Erreurs affichées en haut d'une page

error-guild-not-found = Aucune guilde Hypixel ne porte ce nom.
error-no-hypixel-guild = Lie une guilde Hypixel avant d'ajouter des règles de guilde.
error-missing-value = Cette règle a besoin d'une valeur.
error-unknown-guild-rank = Ce rang n'existe plus dans la guilde Hypixel.
error-guild-ranks-unavailable = Impossible de charger les rangs de la guilde depuis Hypixel, réessaie plus tard.
error-role-not-allowed = Tu ne peux choisir que des rôles en dessous de ton rôle le plus haut.
error-missing-group-name = Un groupe a besoin d'un nom.
error-unknown-group = Ce groupe n'existe plus.

## Vue d'ensemble

overview-intro = Comment la vérification est configurée sur { $server }.
badge-active = Active
badge-not-set-up = Non configurée
badge-linked = Liée
badge-not-linked = Non liée
badge-none = Aucune
badge-on = Activé
badge-off = Désactivé
badge-rules-active =
    { $count ->
        [one] { $count } active
       *[other] { $count } actives
    }
overview-verification = Vérification
overview-verified-role-hint = Donné par /verify
overview-no-verified-role = Choisis un rôle vérifié pour activer /verify.
overview-hypixel-guild = Guilde Hypixel
overview-no-guild = Lie-en une pour utiliser les règles et tags de guilde.
overview-rules = Règles de rôles
overview-rule-count =
    { $count ->
        [one] { $count } règle
       *[other] { $count } règles
    }
overview-rules-hint = Rôles donnés selon le rang et la guilde Hypixel.
overview-nickname = Pseudo
overview-nickname-hint = Exemple de membre renommé.
overview-configure = Configurer →
overview-manage = Gérer →
overview-how-title = Comment les membres se vérifient
overview-how-hint = Ce qu'il faut dire à tes membres.
overview-how-link = Sur Hypixel, ouvrir le menu du profil → Social Media → Discord, et y entrer son pseudo Discord.
overview-how-verify = Sur ce serveur, lancer /verify avec son pseudo Minecraft.
overview-how-roles = OBot vérifie le lien, puis met à jour ses rôles.
overview-how-roles-nickname = OBot vérifie le lien, puis met à jour ses rôles et son pseudo.

## Page Vérification

verification-intro = Ce que /verify donne et retire, et qui est vérifié.
verification-roles = Rôles de vérification
verification-roles-hint = Les rôles grisés sont au-dessus du rôle le plus haut du bot ou du tien.
verified-role = Rôle vérifié (donné)
verified-role-none = Aucun : /verify désactivé
unverified-role = Rôle non vérifié (retiré)
role-none = Aucun
save-roles = Enregistrer les rôles
hypixel-guild = Guilde Hypixel
hypixel-guild-hint = Nécessaire pour les règles de membre et de rang de guilde, et pour les tags de guilde dans les pseudos. Laisser vide pour délier.
guild-name = Nom de la guilde
guild-not-linked = Non liée
save-guild = Enregistrer la guilde

## Tableau des membres

members = Membres
members-count = { $verified } sur { $total } vérifiés
members-matching =
    { $count ->
        [one] { $count } correspondant
       *[other] { $count } correspondants
    }
members-unavailable = Impossible de lister les membres du serveur. Active Server Members Intent dans le portail développeur Discord (Bot → Privileged Gateway Intents), puis recharge.
members-guild-not-linked = Lie une guilde Hypixel pour remplir les colonnes de guilde.
members-guild-unavailable = Impossible de charger la guilde depuis Hypixel, les colonnes de guilde sont vides.
search = Recherche
search-placeholder = Pseudo Discord ou Minecraft, puis Entrée
columns = Colonnes
visible-columns = Colonnes affichées
column-name = Membre
column-minecraft = Minecraft
column-status = Statut
column-joined = Arrivée sur le serveur
column-verified = Vérifié le
column-guild-joined = Arrivée dans la guilde
column-xp = XP de guilde (7 jours)
column-actions = Actions
no-member-matches = Aucun membre ne correspond.
member-pages = Pages de membres
page-previous = ← Précédente
page-next = Suivante →
page-of = Page { $page } sur { $pages }
status-verified = Vérifié
status-verified-by-admin = Vérifié par un admin
status-role-only = Rôle seul
status-role-only-hint = A le rôle vérifié mais pas de compte Minecraft lié, par exemple vérifié avant que les comptes soient enregistrés
status-not-verified = Non vérifié
pick-verified-role-first = Choisis d'abord un rôle vérifié
reverify = Revérifier
verify-member = Vérifier…
remove = Retirer
remove-confirm = Retirer la vérification de { $name } ? Ses rôles de vérification et de règles sont retirés et son pseudo est réinitialisé.
verify-dialog-hint = Même contrôle que /verify : le compte Minecraft doit avoir le Discord de ce membre lié sur Hypixel. Ses rôles et son pseudo sont ensuite appliqués.
minecraft-username = Pseudo Minecraft
cancel = Annuler
verify = Vérifier

## Actions sur les membres : résultats et refus

notice-reverified = Revérifié en tant que { $name }.
notice-verified-by-you = Vérifié en tant que { $name } par toi.
notice-removed = Vérification retirée.
notice-added = Ajouté : { $roles }.
notice-removed-roles = Retiré : { $roles }.
notice-nickname-set = Pseudo changé en { $nickname }.
notice-nickname-reset = Pseudo réinitialisé.
notice-nickname-skipped = Pseudo non changé en { $nickname } : { $reason }.
notice-up-to-date = Tout était déjà à jour.
notice-no-roles-left = Il n'avait plus de rôle de vérification.
deleted-role = rôle supprimé
no-linked-account = Ce membre n'a pas de compte lié, utilise Vérifier à la place.
account-gone = Le compte Minecraft { $name } n'existe plus.
cannot-manage-server = Tu ne peux plus gérer ce serveur.
member-left = Ce membre a quitté le serveur.
bots-cannot-verify = Les bots ne peuvent pas être vérifiés.
enter-username = Entre un pseudo Minecraft.
unknown-account = Aucun compte Minecraft ne s'appelle { $name }.
never-joined = { $name } ne s'est jamais connecté à Hypixel.
no-discord-linked = { $name } n'a pas de Discord lié sur Hypixel. Demande d'abord à { $member } de lier @{ $member } dans le menu Social Media du jeu.
linked-to-someone-else = { $name } est lié au Discord { $linked } sur Hypixel, pas à @{ $member }.
member-already-verified = @{ $member } est déjà vérifié en tant que { $name }. Retire d'abord sa vérification.
account-taken = { $name } est déjà lié à un autre membre ({ $member }).
server-unavailable = Le bot n'a pas pu charger ce serveur.
owner-only = Seul le propriétaire du serveur peut retirer sa propre vérification.
member-above-you = Le rôle le plus haut de ce membre est au niveau du tien ou au-dessus.

## Messages du navigateur

error-session-expired = Ta session a expiré, recharge la page pour te reconnecter.
error-generic = Une erreur est survenue, consulte les logs du bot.
error-unreachable = Impossible de joindre le panel, réessaie.

## Page Règles de rôles

rules-intro = Vérifiées à chaque /verify : le rôle d'une règle est donné quand elle correspond et retiré sinon. Le rôle séparateur d'un groupe est donné tant qu'une de ses règles correspond.
rules = Règles
rules-summary =
    { $count ->
        [one] { $count } règle
       *[other] { $count } règles
    }
rules-in-groups =
    { $groups ->
        [one] dans { $groups } groupe
       *[other] dans { $groups } groupes
    }
rules-empty = Aucune règle pour l'instant. Ajoutes-en une ci-dessous.
group-separator = séparateur
delete-group = Supprimer le groupe
delete-group-confirm = Supprimer le groupe { $name } ? Ses règles sont gardées, sans groupe.
group-empty = Aucune règle dans ce groupe pour l'instant : son séparateur n'est jamais donné.
no-group = Sans groupe
delete-rule = Supprimer la règle
edit-rule = Modifier la règle
save-rule = Enregistrer la règle
edit-group = Modifier le groupe
save-group = Enregistrer le groupe
drag-rule = Glisser vers un autre groupe
rules-drag-hint = Glisse une règle sur un groupe pour l'y déplacer.
no-group-empty = Dépose une règle ici pour la sortir de son groupe.
add-rule = Ajouter une règle
add-rule-no-guild = Lie une guilde Hypixel dans Paramètres généraux pour utiliser les conditions de guilde.
add-rule-guild = Les conditions de guilde utilisent { $guild }.
rule-when = Quand
rule-kind-hypixel-rank = Le rang Hypixel est
rule-kind-guild-member = Membre de la guilde Hypixel
rule-kind-guild-rank = Le rang de guilde est
hypixel-rank = Rang Hypixel
guild-rank = Rang de guilde
guild-ranks-unavailable = Impossible de charger les rangs depuis Hypixel
guild-ranks-link-first = Lie d'abord une guilde Hypixel
guild-ranks-none = Aucun rang trouvé
give-role = Donner le rôle
group = Groupe
add-rule-button = Ajouter la règle
add-group = Ajouter un groupe
add-group-hint = Regroupe des règles sous un rôle séparateur, comme ━━ Rangs ━━ placé au-dessus des rôles de rang. Les membres ont le séparateur tant qu'une règle du groupe correspond.
group-name = Nom
group-name-placeholder = ex. Rangs
separator-role = Rôle séparateur
add-group-button = Ajouter le groupe
condition-no-rank = Pas de rang Hypixel
condition-hypixel-rank = Le rang Hypixel est { $rank }
condition-guild-member = Membre de la guilde Hypixel
condition-guild-rank = Le rang de guilde est { $rank }
rank-no-rank = Pas de rang

## Page Pseudo

nickname-intro = Renomme les membres à chaque /verify selon leur profil Hypixel.
nickname-enabled = Renommer les membres à chaque /verify
separator = Séparateur entre les champs
fields = Champs
fields-hint = Les champs sans valeur (pas de rang, pas dans la guilde) sont ignorés avec leur préfixe et suffixe. Au-delà de 32 caractères, le numéro d'importance le plus haut est retiré en premier.
field-show = Afficher
field = Champ
field-order = Ordre
field-prefix = Préfixe
field-suffix = Suffixe
field-importance = Importance
field-hypixel-rank = Rang Hypixel
field-ign = Pseudo Minecraft
field-guild-rank-tag = Tag du rang de guilde
field-guild-tag = Tag de guilde
texts-per-rank = Textes par rang
texts-per-rank-hint = Change le préfixe, le libellé ou le suffixe d'un seul rang, comme ★MVP++★. Les parties non modifiées suivent les valeurs par défaut du champ ci-dessus ; une partie vidée n'affiche rien. Les rangs sans libellé sont ignorés.
texts-hypixel-ranks = Rangs Hypixel
texts-guild-ranks = Rangs de guilde
texts-link-guild = Lie une guilde Hypixel dans Paramètres généraux pour personnaliser ses rangs.
texts-guild-gone = La guilde Hypixel liée n'existe plus.
texts-guild-unavailable = Impossible de charger les rangs de la guilde depuis Hypixel, réessaie plus tard.
rank = Rang
label = Libellé
label-none = aucun
custom = Personnalisé
preview = Aperçu
reset-text = Revenir aux textes du champ
hide-text = Ne rien afficher pour ce rang
reset-all-texts = Tout réinitialiser
text-actions = Actions
preview-hint = Se met à jour pendant l'édition. « Le plus long » a chaque champ à sa longueur maximale.
preview-example = Exemple
preview-longest = Le plus long
preview-unchanged = (inchangé)
save-nickname = Enregistrer le pseudo

## Page Paramètres généraux

nav-settings = Paramètres généraux
settings-intro = Les réglages de tout le serveur : sa guilde Hypixel, et ce qu'OBot y publie.
settings-messages = Langue et logs
settings-messages-hint = Ce qu'OBot publie sur le serveur est écrit dans la langue du serveur. Les réponses aux commandes suivent la langue Discord de chaque membre.
server-language = Langue du serveur
log-channel = Salon de logs
log-channel-none = Aucun salon de logs
log-channel-hint = Les vérifications, les mises à jour qui ont changé quelque chose et les actions du panel y sont publiées. Les salons grisés sont ceux où le bot ne peut pas écrire.
save-settings = Enregistrer
error-channel-not-sendable = Le bot ne peut pas envoyer de messages dans ce salon.
verify-button-card = Bouton de vérification
verify-button-hint = Publie un message avec un bouton Vérifier, dans la langue du serveur. Les membres cliquent dessus et entrent leur pseudo Minecraft au lieu de taper /verify. Les anciens messages continuent de fonctionner ; supprime-les depuis Discord.
verify-button-channel = Salon
post-verify-message = Publier le message
notice-verify-message-posted = Le message de vérification a été publié.
error-post-failed = Discord a refusé le message, vérifie les permissions du bot dans ce salon.

## Demandes RGPD, réservées aux admins confidentialité de l'instance (PRIVACY_ADMIN_IDS)

data-requests = Demandes RGPD
data-requests-intro = Retrouve tout ce qu'OBot enregistre sur une personne, sur tous les serveurs, et efface-le à sa demande (droits d'accès et à l'effacement du RGPD).
data-search = Rechercher
data-search-placeholder = ID Discord, pseudo ou UUID Minecraft
data-search-hint = Les ID Discord et les UUID Minecraft doivent correspondre exactement ; les pseudos Minecraft et les noms du panel, en partie.
data-no-results = Rien d'enregistré ne correspond à « { $query } ».
data-unknown-user = Utilisateur Discord inconnu
data-person-sessions = Sessions du panel : { $count }
data-person-verified-by-them = Membres vérifiés en tant qu'admin : { $count }
data-person-none = Aucune vérification.
data-column-server = Serveur
data-column-account = Compte Minecraft
data-column-date = Vérifié le
data-column-by = Vérifié par
data-by-self = Lui-même
data-erase = Effacer toutes ses données
data-erase-hint = Supprime ses vérifications et ses sessions du panel, et retire son ID des vérifications qu'il a faites en tant qu'admin. Ses rôles et son pseudo sur Discord, et les messages déjà publiés dans les salons de logs, restent : ils appartiennent à chaque serveur.
data-erase-confirm = Effacer toutes les données de { $user } ? C'est irréversible.
notice-data-erased = Données de { $user } effacées : vérifications : { $verifications }, sessions : { $sessions }, mentions en tant qu'admin : { $admin }.
