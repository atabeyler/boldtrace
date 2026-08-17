welcome-greeting = Bienvenue sur Boldtrace !
welcome-choose-language = Veuillez choisir votre langue pour continuer.

language-prompt = Veuillez sélectionner votre langue :
language-changed = Langue définie sur { $language }.

consent-title = Conditions et avertissement
consent-body = Boldtrace fournit uniquement des signaux de marché statistiques. Ceci ne constitue pas un conseil en investissement. En continuant, vous acceptez les conditions d'utilisation et cet avertissement.
consent-accept-button = J'accepte
consent-required = Veuillez accepter les conditions avant d'utiliser cette commande. Envoyez /start pour commencer.

help-title = Commandes disponibles
help-tara = /tara <SYMBOLE> - Affiche le score actuel d'un symbole
help-alarm = /alarm <SYMBOLE> <SEUIL> - Recevez une notification lorsque le score d'un symbole dépasse un seuil
help-language = /language - Changer de langue
help-help = /help - Afficher ce message

tara-usage = Utilisation : /tara <SYMBOLE>
tara-no-data = Aucune donnée de marché n'est encore disponible pour { $symbol }. Veuillez réessayer sous peu.
tara-result =
    Score pour { $symbol } : { $score }/100

    Anomalie de volume : { $volume_anomaly }
    Extrémité du taux de financement : { $funding_extreme }
    Déséquilibre du carnet d'ordres : { $order_book_imbalance }
    Divergence RSI : { $rsi_divergence }

alarm-usage = Utilisation : /alarm <SYMBOLE> <SEUIL>
alarm-invalid-threshold = Le seuil doit être un nombre compris entre 0 et 100.
alarm-set = Alarme définie : vous serez averti lorsque le score de { $symbol } dépassera { $threshold }.
alarm-triggered = Le score de { $symbol } a dépassé { $threshold } : score actuel { $score }.

footer-company = Bold A.S. © 2026
footer-rights = Tous droits réservés.
footer-disclaimer = Ceci est une probabilité statistique, pas un conseil en investissement.

error-generic = Une erreur s'est produite. Veuillez réessayer plus tard.

language-name-en = English
language-name-tr = Türkçe
language-name-fr = Français
language-name-de = Deutsch
language-name-ar = العربية
language-name-ru = Русский
