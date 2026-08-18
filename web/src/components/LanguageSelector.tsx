import {useI18n,type Lang} from '../i18n';
const langs:[Lang,string][]=[['tr','Türkçe'],['en','English'],['de','Deutsch'],['fr','Français'],['ar','العربية'],['ru','Русский']];
export function LanguageSelector(){const{lang,setLang}=useI18n();return <select className="language-select" value={lang} onChange={e=>setLang(e.target.value as Lang)} aria-label="Language">{langs.map(([code,label])=><option key={code} value={code}>{label}</option>)}</select>}
