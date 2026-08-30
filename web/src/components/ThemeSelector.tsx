import { useI18n } from '../i18n';
import { useTheme, type ThemePreference } from '../theme';

const labels:Record<string,Record<ThemePreference,string>>={
  tr:{light:'Açık',dark:'Koyu',system:'Sistem'},en:{light:'Light',dark:'Dark',system:'System'},
  de:{light:'Hell',dark:'Dunkel',system:'System'},fr:{light:'Clair',dark:'Sombre',system:'Système'},
  ar:{light:'فاتح',dark:'داكن',system:'النظام'},ru:{light:'Светлая',dark:'Тёмная',system:'Система'},
};

export function ThemeSelector(){
  const{lang}=useI18n();
  const{preference,setPreference,effectiveTheme}=useTheme();
  const copy=labels[lang]??labels.en;
  const choices:{id:ThemePreference;icon:string}[]=[{id:'light',icon:'☀'},{id:'dark',icon:'☾'},{id:'system',icon:'◐'}];
  return <div className="theme-selector" role="radiogroup">
    {choices.map(choice=><button type="button" role="radio" aria-checked={preference===choice.id} className={preference===choice.id?'active':''} key={choice.id} onClick={()=>setPreference(choice.id)}><span>{choice.icon}</span><b>{copy[choice.id]}</b>{choice.id==='system'&&<small>{effectiveTheme==='light'?copy.light:copy.dark}</small>}</button>)}
  </div>;
}
