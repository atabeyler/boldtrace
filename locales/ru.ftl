welcome-greeting = Добро пожаловать в Boldtrace!
welcome-choose-language = Пожалуйста, выберите язык, чтобы продолжить.

language-prompt = Пожалуйста, выберите язык:
language-changed = Язык изменён на { $language }.

consent-title = Условия и отказ от ответственности
consent-body = Boldtrace предоставляет только статистические рыночные сигналы. Это не является инвестиционной рекомендацией. Продолжая, вы принимаете условия использования и этот отказ от ответственности.
consent-accept-button = Я согласен
consent-required = Пожалуйста, примите условия перед использованием этой команды. Отправьте /start, чтобы начать.

help-title = Доступные команды
help-tara = /tara <СИМВОЛ> - Показать текущий скор для символа
help-alarm = /alarm <СИМВОЛ> <ПОРОГ> - Получать уведомление при пересечении скором символа порога
help-language = /language - Изменить язык
help-help = /help - Показать это сообщение

tara-usage = Использование: /tara <СИМВОЛ>
tara-no-data = Данные по { $symbol } пока недоступны. Пожалуйста, попробуйте снова чуть позже.
tara-result =
    Скор { $symbol }: { $score }/100

    Аномалия объёма: { $volume_anomaly }
    Экстремальность ставки финансирования: { $funding_extreme }
    Дисбаланс стакана заявок: { $order_book_imbalance }
    Дивергенция RSI: { $rsi_divergence }

alarm-usage = Использование: /alarm <СИМВОЛ> <ПОРОГ>
alarm-invalid-threshold = Порог должен быть числом от 0 до 100.
alarm-set = Аларм установлен: вы получите уведомление, когда скор { $symbol } пересечёт { $threshold }.
alarm-triggered = Скор { $symbol } пересёк { $threshold }: текущий скор { $score }.

footer-company = Bold A.S. © 2026
footer-rights = Все права защищены.
footer-disclaimer = Это статистическая вероятность, а не инвестиционная рекомендация.

error-generic = Что-то пошло не так. Пожалуйста, попробуйте позже.

language-name-en = English
language-name-tr = Türkçe
language-name-fr = Français
language-name-de = Deutsch
language-name-ar = العربية
language-name-ru = Русский
