export const navigation=[
{id:'command',label:'Command Center',icon:'⌁'},
{id:'intelligence',label:'Intelligence Terminal',icon:'◈'},
{id:'engines',label:'Engine Matrix',icon:'▦'},
{id:'performance',label:'Performance',icon:'↗'},
{id:'learning',label:'Learning Center',icon:'◎'},
{id:'scanner',label:'Market Scanner',icon:'⌖'},
{id:'alerts',label:'Alerts',icon:'◉'},
{id:'history',label:'History',icon:'◷'},
{id:'health',label:'System Health',icon:'◇'},
{id:'settings',label:'Settings',icon:'⚙'}] as const;
export const settingsSections=[
{id:'profile',title:'Profile & Account',items:['Name','Email','Password','Two-factor authentication','Active sessions']},
{id:'appearance',title:'Appearance',items:['Theme','Interface density','Chart appearance','Reduced motion']},
{id:'language',title:'Language & Region',items:['Language','Timezone','Number format']},
{id:'markets',title:'Markets',items:['Watchlist','Default market','Quote currency','Scanner universe']},
{id:'intelligence',title:'Intelligence',items:['Default horizon','Confidence threshold','Maximum risk','Show engine details','Adaptive learning status']},
{id:'notifications',title:'Notifications',items:['In-app alerts','Telegram alerts','Email alerts','Alert severity','Quiet hours']},
{id:'privacy',title:'Privacy & Security',items:['Two-factor authentication','Login activity','Export account data','Delete account']},
{id:'system',title:'System',items:['Data health','API status','Build version','Diagnostics']}
] as const;
