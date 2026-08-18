import {useI18n} from '../i18n';import {Brand} from './Brand';
export function Footer(){const{t}=useI18n();return <footer className="footer"><Brand compact/><div><b>{t.footer}</b><span> © {new Date().getFullYear()} BOLD</span></div><span className="disclaimer">{t.disclaimer}</span></footer>}
