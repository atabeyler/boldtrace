welcome-greeting = Willkommen bei Boldtrace!
welcome-choose-language = Bitte wählen Sie Ihre Sprache, um fortzufahren.

language-prompt = Bitte wählen Sie Ihre Sprache:
language-changed = Sprache auf { $language } eingestellt.

consent-title = Nutzungsbedingungen und Haftungsausschluss
consent-body = Boldtrace liefert ausschließlich statistische Marktsignale. Dies ist keine Anlageberatung. Durch die Fortsetzung akzeptieren Sie die Nutzungsbedingungen und diesen Haftungsausschluss.
consent-accept-button = Ich stimme zu
consent-required = Bitte akzeptieren Sie die Bedingungen, bevor Sie diesen Befehl verwenden. Senden Sie /start, um zu beginnen.

help-title = Verfügbare Befehle
help-tara = /tara <SYMBOL> - Zeigt die aktuelle Punktzahl für ein Symbol
help-alarm = /alarm <SYMBOL> <SCHWELLE> - Benachrichtigung erhalten, wenn die Punktzahl eines Symbols eine Schwelle überschreitet
help-language = /language - Sprache ändern
help-help = /help - Diese Nachricht anzeigen

tara-usage = Verwendung: /tara <SYMBOL>
tara-no-data = Für { $symbol } sind noch keine Marktdaten verfügbar. Bitte versuchen Sie es in Kürze erneut.
tara-result =
    Punktzahl für { $symbol }: { $score }/100

    Volumenanomalie: { $volume_anomaly }
    Funding-Rate-Extremität: { $funding_extreme }
    Orderbuch-Ungleichgewicht: { $order_book_imbalance }
    RSI-Divergenz: { $rsi_divergence }

alarm-usage = Verwendung: /alarm <SYMBOL> <SCHWELLE>
alarm-invalid-threshold = Die Schwelle muss eine Zahl zwischen 0 und 100 sein.
alarm-set = Alarm gesetzt: Sie werden benachrichtigt, wenn die Punktzahl von { $symbol } { $threshold } überschreitet.
alarm-triggered = Die Punktzahl von { $symbol } hat { $threshold } überschritten: aktuelle Punktzahl { $score }.

footer-company = Bold A.S. © 2026
footer-rights = Alle Rechte vorbehalten.
footer-disclaimer = Dies ist eine statistische Wahrscheinlichkeit, keine Anlageberatung.

error-generic = Etwas ist schiefgelaufen. Bitte versuchen Sie es später erneut.

language-name-en = English
language-name-tr = Türkçe
language-name-fr = Français
language-name-de = Deutsch
language-name-ar = العربية
language-name-ru = Русский
