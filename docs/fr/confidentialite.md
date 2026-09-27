# Politique de confidentialité

*Dernière mise à jour : 27 septembre 2026* · [English version](../privacy.md)

Cette politique explique quelles données OBot traite, pourquoi, combien de temps elles sont
gardées et comment exercer vos droits. La version française fait foi.

## 1. Qui est responsable

OBot est un bot Discord libre, publié sous licence BSD 3-Clause
([code source](https://github.com/Oblivion-FR/OBot)).

- **Instance officielle** : elle est opérée par **Oblivion FR**, responsable du traitement au
  sens du Règlement général sur la protection des données (RGPD). Contact :
  **privacy@oblivion.ovh**.
- **Instances auto-hébergées** : toute personne peut faire tourner sa propre copie d'OBot. Elle
  en est alors seule responsable, et cette politique ne décrit que ce que fait le logiciel.
  Oblivion FR n'a aucun accès aux données de ces instances.

## 2. Données traitées

OBot ne collecte que ce qui sert à la vérification des comptes Minecraft et à son panel web. Il
ne lit pas le contenu des messages, n'utilise aucun outil de mesure d'audience et ne revend
aucune donnée.

### Membres qui se vérifient

Quand vous utilisez `/verify` ou le bouton de vérification, ou qu'un administrateur vous vérifie
depuis le panel, OBot enregistre :

- votre identifiant Discord et celui du serveur ;
- l'identifiant (UUID) et le pseudo du compte Minecraft que vous avez prouvé posséder ;
- la date de la vérification et, si elle a été faite par un administrateur, son identifiant
  Discord.

Pour vérifier et tenir vos rôles à jour, OBot lit aussi, sans les enregistrer, votre profil
Hypixel (le Discord qui y est lié, votre rang Hypixel), votre guilde Hypixel et votre rang dans
celle-ci. Ces réponses sont gardées en mémoire au plus 10 minutes. OBot modifie ensuite vos
rôles et, si le serveur l'a activé, votre pseudo sur le serveur. Ces informations sont
actualisées automatiquement toutes les 3 heures environ.

### Administrateurs qui utilisent le panel

La connexion au panel passe par Discord, avec la seule autorisation `identify` : OBot ne reçoit
ni votre adresse e-mail, ni la liste de vos serveurs. Il enregistre, le temps de la session :

- votre identifiant, votre nom affiché, le lien de votre avatar et la langue de votre compte
  Discord ;
- une empreinte (SHA-256) du jeton de session, jamais le jeton lui-même.

Pour afficher le tableau des membres, le panel lit la liste des membres du serveur depuis
Discord (nom, avatar, rôles, date d'arrivée) et l'XP de guilde Hypixel des 7 derniers jours. Ces
données sont gardées en mémoire au plus quelques minutes et ne sont pas enregistrées.

### Serveurs

Pour chaque serveur, OBot enregistre sa configuration : rôles choisis, guilde Hypixel liée,
règles de rôles, format des pseudos, langue du serveur et salon de logs.

### Salon de logs

Si un serveur a choisi un salon de logs, OBot y publie les vérifications, les refus de
vérification, les actions faites depuis le panel et les mises à jour automatiques, avec les
membres concernés. Ces messages sont stockés par Discord : ils sont visibles des personnes qui
ont accès au salon, et leur suppression relève des administrateurs du serveur.

### Journaux techniques

En cas d'erreur, le serveur qui héberge OBot peut enregistrer dans ses journaux des identifiants
Discord et des pseudos Minecraft, uniquement pour le diagnostic.

## 3. Finalités et bases légales

| Finalité                                            | Base légale (article 6 du RGPD)                          |
|-----------------------------------------------------|----------------------------------------------------------|
| Vérifier un compte Minecraft et attribuer les rôles | Exécution du service que vous demandez en vous vérifiant |
| Mettre à jour les rôles et pseudos régulièrement    | Intérêt légitime du serveur à garder des rôles exacts    |
| Permettre aux administrateurs de gérer leur serveur | Exécution du service                                     |
| Publier dans le salon de logs                       | Intérêt légitime du serveur à suivre les vérifications   |
| Sécurité, prévention des abus, diagnostic           | Intérêt légitime de l'opérateur                          |

## 4. Cookies

Le panel n'utilise que des cookies nécessaires à son fonctionnement ou à vos préférences, sans
pistage ni publicité. Ils sont donc dispensés de consentement.

| Cookie                | Rôle                                     | Durée                              |
|-----------------------|------------------------------------------|------------------------------------|
| `obot_session`        | Vous garder connecté                     | Session du navigateur, 7 jours max |
| `obot_oauth_state`    | Protéger la connexion via Discord        | Le temps de la connexion           |
| `obot_lang`           | Retenir la langue choisie                | 1 an                               |
| `obot_hidden_columns` | Retenir les colonnes masquées du tableau | 1 an                               |

## 5. Destinataires et services tiers

Vos données ne sont communiquées qu'aux services nécessaires au fonctionnement d'OBot :

- **Discord** : connexion au panel, lecture des membres, attribution des rôles et pseudos,
  messages ;
- **Mojang / Microsoft** : conversion d'un pseudo Minecraft en identifiant, et inversement ;
- **Hypixel** : lecture des profils et des guildes, à partir de l'identifiant Minecraft ;
- **jsDelivr** : le panel charge la bibliothèque htmx depuis ce service, qui reçoit alors
  l'adresse IP du visiteur ; les avatars sont chargés depuis le réseau de Discord.

Ces services peuvent traiter des données hors de l'Union européenne, notamment aux États-Unis,
selon leurs propres politiques de confidentialité. Les administrateurs d'un serveur voient, dans
le panel et le salon de logs, les données des membres de leur serveur.

## 6. Durées de conservation

- **Vérification d'un membre** : tant qu'elle n'est pas retirée. Un administrateur peut la
  retirer depuis le panel. Si vous quittez le serveur, elle est gardée pour vous rendre vos
  rôles si vous revenez, et supprimée si quelqu'un d'autre vérifie ensuite le même compte
  Minecraft.
- **Session du panel** : 7 jours au plus, ou jusqu'à la déconnexion.
- **Configuration d'un serveur** : tant qu'elle n'est pas supprimée, y compris si le bot est
  retiré du serveur. Vous pouvez demander sa suppression.
- **Données en mémoire** : quelques minutes, et perdues à chaque redémarrage.

## 7. Vos droits

Conformément au RGPD, vous disposez d'un droit d'accès, de rectification, d'effacement, de
limitation, d'opposition et de portabilité sur vos données. Pour les exercer, écrivez à
**privacy@oblivion.ovh** depuis une adresse qui permet de vous répondre, en indiquant votre
identifiant Discord. Une réponse vous est apportée sous un mois.

Un administrateur de votre serveur peut aussi retirer votre vérification immédiatement depuis le
panel.

Si vous estimez que vos droits ne sont pas respectés, vous pouvez adresser une réclamation à la
CNIL ([cnil.fr](https://www.cnil.fr/fr/plaintes)).

## 8. Sécurité

Le panel est servi en HTTPS, les sessions sont stockées sous forme d'empreinte, et chaque action
vérifie à nouveau, auprès de Discord, que vous avez les permissions nécessaires sur le serveur.

## 9. Mineurs

OBot s'adresse aux utilisateurs de Discord, qui doivent avoir l'âge minimum fixé par les
conditions de Discord dans leur pays.

## 10. Modifications

Cette politique peut évoluer avec le logiciel. Chaque version est publiée dans le
[dépôt du projet](https://github.com/Oblivion-FR/OBot), avec l'historique de ses changements.
