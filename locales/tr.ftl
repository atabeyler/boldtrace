welcome-greeting = Boldtrace'e hoş geldiniz!
welcome-choose-language = Devam etmek için lütfen dilinizi seçin.

language-prompt = Lütfen dilinizi seçin:
language-changed = Dil { $language } olarak ayarlandı.

consent-title = Şartlar ve Uyarı
consent-body = Boldtrace yalnızca istatistiksel piyasa sinyalleri sunar. Bu bir yatırım tavsiyesi değildir. Devam ederek Kullanım Şartlarını ve bu uyarıyı kabul etmiş olursunuz.
consent-accept-button = Kabul Ediyorum
consent-required = Bu komutu kullanmadan önce lütfen şartları kabul edin. Başlamak için /start yazın.

help-title = Kullanılabilir komutlar
help-tara = /tara <SEMBOL> - Bir sembolün güncel skorunu gösterir
help-alarm = /alarm <SEMBOL> <EŞİK> - Bir sembolün skoru eşiği geçtiğinde bildirim alın
help-language = /language - Dilinizi değiştirin
help-help = /help - Bu mesajı gösterir

tara-usage = Kullanım: /tara <SEMBOL>
tara-no-data = { $symbol } için henüz piyasa verisi yok. Lütfen kısa süre sonra tekrar deneyin.
tara-result =
    { $symbol } skoru: { $score }/100

    Hacim anomalisi: { $volume_anomaly }
    Funding rate aşırılığı: { $funding_extreme }
    Order book dengesizliği: { $order_book_imbalance }
    RSI sapması: { $rsi_divergence }

alarm-usage = Kullanım: /alarm <SEMBOL> <EŞİK>
alarm-invalid-threshold = Eşik 0 ile 100 arasında bir sayı olmalıdır.
alarm-set = Alarm kuruldu: { $symbol } skoru { $threshold } eşiğini geçtiğinde bilgilendirileceksiniz.
alarm-triggered = { $symbol } skoru { $threshold } eşiğini geçti: güncel skor { $score }.

footer-company = Bold A.Ş. © 2026
footer-rights = Tüm hakları saklıdır.
footer-disclaimer = Bu istatistiksel bir olasılıktır, yatırım tavsiyesi değildir.

error-generic = Bir şeyler ters gitti. Lütfen daha sonra tekrar deneyin.

language-name-en = English
language-name-tr = Türkçe
language-name-fr = Français
language-name-de = Deutsch
language-name-ar = العربية
language-name-ru = Русский
