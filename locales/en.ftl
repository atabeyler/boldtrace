welcome-greeting = Welcome to Boldtrace!
welcome-choose-language = Please choose your language to continue.

language-prompt = Please select your language:
language-changed = Language set to { $language }.

consent-title = Terms & Disclaimer
consent-body = Boldtrace surfaces statistical market signals only. This is not investment advice. By continuing, you accept the Terms of Use and this disclaimer.
consent-accept-button = I Agree
consent-required = Please accept the terms before using this command. Send /start to begin.

help-title = Available commands
help-tara = /tara <SYMBOL> - Show the current score for a symbol
help-alarm = /alarm <SYMBOL> <THRESHOLD> - Get notified when a symbol's score crosses a threshold
help-language = /language - Change your language
help-help = /help - Show this message

tara-usage = Usage: /tara <SYMBOL>
tara-no-data = No market data is available yet for { $symbol }. Please try again shortly.
tara-result =
    Score for { $symbol }: { $score }/100

    Volume anomaly: { $volume_anomaly }
    Funding rate extremity: { $funding_extreme }
    Order book imbalance: { $order_book_imbalance }
    RSI divergence: { $rsi_divergence }

alarm-usage = Usage: /alarm <SYMBOL> <THRESHOLD>
alarm-invalid-threshold = Threshold must be a number between 0 and 100.
alarm-set = Alarm set: you will be notified when { $symbol }'s score crosses { $threshold }.
alarm-triggered = { $symbol } score crossed { $threshold }: current score is { $score }.

footer-company = Bold A.S. (c) 2026
footer-rights = All rights reserved.
footer-disclaimer = This is a statistical probability, not investment advice.

error-generic = Something went wrong. Please try again later.

language-name-en = English
language-name-tr = Türkçe
language-name-fr = Français
language-name-de = Deutsch
language-name-ar = العربية
language-name-ru = Русский
