// Recreate the designer's Intl.NumberFormat reference cases using Node.js.
import {writeFileSync} from 'node:fs';
const numbers=[0,-0,1,-1,0.00001,-0.00001,0.0049,-0.0049,0.005,-0.005,0.12567,1.005,-1.005,2.675,-2.675,999.995,1234.5,-1234.565,12345.675,1000000.25,1e-9,1e20,1e30,1e308,5e-324];
const cases=[];
for(const locale of ['en-US','es-MX'])for(const number of numbers){
 const input=Object.is(number,-0)?'-0':String(number);
 for(const style of ['number','percent']) cases.push({locale,currency:'USD',style,input,expected:new Intl.NumberFormat(locale,style==='number'?{maximumFractionDigits:2}:{style:'percent',maximumFractionDigits:2}).format(number)});
 for(const currency of ['USD','MXN','EUR','CAD','GBP','JPY']) cases.push({locale,currency,style:'currency',input,expected:new Intl.NumberFormat(locale,{style:'currency',currency}).format(number)});
}
writeFileSync(new URL('./intl-number-cases.json',import.meta.url),JSON.stringify(cases,null,2)+'\n');
