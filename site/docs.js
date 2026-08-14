const documents=[
  {title:{en:"Developer manual",pt:"Manual do desenvolvedor"},path:{en:"content/docs/MANUAL_DESENVOLVEDOR_EN.md",pt:"content/docs/MANUAL_DESENVOLVEDOR_PTBR.md"}},
  {title:{en:"Architecture",pt:"Arquitetura"},path:{en:"content/docs/arquitetura/ARQUITETURA.md",pt:"content/docs/arquitetura/ARQUITETURA.md"}},
  {title:{en:"Architecture report",pt:"Relatório de arquitetura"},path:{en:"content/docs/architecture_report.md",pt:"content/docs/architecture_report.md"}},
  {title:{en:"API module",pt:"Módulo de API"},path:{en:"content/docs/api/API_MODULE.md",pt:"content/docs/api/API_MODULE.md"}},
  {title:{en:"Changelog",pt:"Changelog"},path:{en:"content/docs/changelog/CHANGELOG.md",pt:"content/docs/changelog/CHANGELOG.md"}},
  {title:{en:"Release notes",pt:"Notas de lançamento"},path:{en:"content/docs/releases/RELEASE_NOTES.md",pt:"content/docs/releases/RELEASE_NOTES.md"}},
  {title:{en:"Development diary",pt:"Diário de desenvolvimento"},path:{en:"content/docs/diary/README.md",pt:"content/docs/diary/README.md"}}
];
let activeDocument=0;
async function showDocument(index){activeDocument=index;const language=currentLanguage==="pt-BR"?"pt":"en";const documentInfo=documents[index];const viewer=document.querySelector("#document-viewer");document.querySelectorAll("#docs-list button").forEach((button,i)=>button.classList.toggle("active",i===index));viewer.innerHTML=`<p>${t("loading_document")}</p>`;try{const response=await fetch(documentInfo.path[language]);if(!response.ok)throw new Error(response.status);const markdown=await response.text();viewer.innerHTML=DOMPurify.sanitize(marked.parse(markdown));viewer.querySelectorAll("a").forEach(link=>{if(link.host&&link.host!==location.host)link.target="_blank"})}catch(_){viewer.innerHTML="<p>Documentation could not be loaded.</p>"}}
function renderDocumentList(){const language=currentLanguage==="pt-BR"?"pt":"en";const list=document.querySelector("#docs-list");list.innerHTML="";documents.forEach((item,index)=>{const button=document.createElement("button");button.type="button";button.textContent=item.title[language];button.addEventListener("click",()=>showDocument(index));list.appendChild(button)});showDocument(activeDocument)}
document.addEventListener("DOMContentLoaded",renderDocumentList);document.addEventListener("tbx-language",renderDocumentList);
