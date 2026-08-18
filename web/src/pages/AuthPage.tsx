import {useState} from 'react';import {Brand} from '../components/Brand';import {LanguageSelector} from '../components/LanguageSelector';import {Footer} from '../components/Footer';import {useI18n} from '../i18n';import {api,ApiError} from '../api/client';import type {Account} from '../api/contracts';

const ERROR_KEYS:Record<string,'errorInvalidInput'|'errorTermsRequired'|'errorEmailTaken'|'errorInvalidCredentials'|'errorServiceUnavailable'>={
  invalid_input:'errorInvalidInput',
  terms_not_accepted:'errorTermsRequired',
  email_taken:'errorEmailTaken',
  invalid_credentials:'errorInvalidCredentials',
  database_unavailable:'errorServiceUnavailable',
};

export function AuthPage({onAuthenticated}:{onAuthenticated:(account:Account)=>void}){
  const{t,lang}=useI18n();
  const[mode,setMode]=useState<'login'|'register'>('login');
  const[busy,setBusy]=useState(false);
  const[error,setError]=useState<string|null>(null);

  const submit=async(e:React.FormEvent<HTMLFormElement>)=>{
    e.preventDefault();
    setError(null);
    setBusy(true);
    const form=new FormData(e.currentTarget);
    try{
      const account=mode==='login'
        ?await api.login({
            email:String(form.get('email')||''),
            password:String(form.get('password')||''),
            rememberMe:form.get('rememberMe')==='on',
          })
        :await api.register({
            firstName:String(form.get('firstName')||''),
            lastName:String(form.get('lastName')||''),
            email:String(form.get('email')||''),
            password:String(form.get('password')||''),
            language:lang,
            termsAccepted:form.get('termsAccepted')==='on',
          });
      onAuthenticated(account);
    }catch(err){
      const code=err instanceof ApiError?err.code:'errorGeneric';
      setError(t[ERROR_KEYS[code]??'errorGeneric']);
    }finally{
      setBusy(false);
    }
  };

  return <div className="auth-page"><div className="auth-grid"/><header className="auth-header"><Brand/><LanguageSelector/></header><section className="auth-hero"><div className="eyebrow"><span/>BOLDTRACE / INTELLIGENCE ACCESS</div><h1>{t.welcome}</h1><p>{t.subtitle}</p><div className="boot-strip"><span>MARKET FEEDS <b>ONLINE</b></span><span>RISK GUARDIAN <b>ACTIVE</b></span><span>ADAPTIVE LEARNING <b>READY</b></span></div></section><section className="auth-panel"><div className="auth-card"><div className="auth-tabs"><button type="button" className={mode==='login'?'active':''} onClick={()=>{setMode('login');setError(null)}}>{t.login}</button><button type="button" className={mode==='register'?'active':''} onClick={()=>{setMode('register');setError(null)}}>{t.register}</button></div><form onSubmit={submit}>{mode==='register'&&<><label>{t.firstName}<input required name="firstName" autoComplete="given-name" placeholder={t.firstName}/></label><label>{t.lastName}<input required name="lastName" autoComplete="family-name" placeholder={t.lastName}/></label></>}<label>{t.email}<input required name="email" type="email" autoComplete="email" placeholder="operator@boldtrace.ai"/></label><label>{t.password}<input required name="password" type="password" minLength={8} autoComplete={mode==='login'?'current-password':'new-password'} placeholder="••••••••••••"/></label>{mode==='login'&&<div className="auth-row"><label className="check"><input type="checkbox" name="rememberMe"/> {t.rememberMe}</label></div>}{mode==='register'&&<label className="check terms-check"><input required type="checkbox" name="termsAccepted"/> {t.termsCheckbox}</label>}{error&&<p className="auth-error" role="alert">{error}</p>}<button className="primary-action" disabled={busy}>{busy?t.authorizing:mode==='login'?t.login:t.register}</button></form><div className="security-note"><span>◇</span><p><b>Encrypted access</b><br/>Session and account controls are protected by the BOLDTRACE security layer.</p></div></div></section><Footer/></div>;
}
