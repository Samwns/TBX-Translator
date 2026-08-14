const documents=[
  {title:{en:"Developer manual",pt:"Manual do desenvolvedor"},path:{en:"content/docs/MANUAL_DESENVOLVEDOR_EN.md",pt:"content/docs/MANUAL_DESENVOLVEDOR_PTBR.md"}},
  {title:{en:"Architecture [EN]",pt:"Arquitetura [EN]"},path:{en:"content/docs/arquitetura/ARQUITETURA.md",pt:"content/docs/arquitetura/ARQUITETURA.md"}},
  {title:{en:"Architecture report [EN]",pt:"Relatório de arquitetura [EN]"},path:{en:"content/docs/architecture_report.md",pt:"content/docs/architecture_report.md"}},
  {title:{en:"API module [EN]",pt:"Módulo de API [EN]"},path:{en:"content/docs/api/API_MODULE.md",pt:"content/docs/api/API_MODULE.md"}},
  {title:{en:"Changelog [PT-BR]",pt:"Changelog [PT-BR]"},path:{en:"content/docs/changelog/CHANGELOG.md",pt:"content/docs/changelog/CHANGELOG.md"}},
  {title:{en:"Release notes [EN]",pt:"Notas de lançamento [EN]"},path:{en:"content/docs/releases/RELEASE_NOTES.md",pt:"content/docs/releases/RELEASE_NOTES.md"}},
  {title:{en:"Development diary [EN]",pt:"Diário de desenvolvimento [EN]"},path:{en:"content/docs/diary/README.md",pt:"content/docs/diary/README.md"}}
];
let activeDocument=0;
async function showDocument(index){activeDocument=index;const language=currentLanguage==="pt-BR"?"pt":"en";const documentInfo=documents[index];const viewer=document.querySelector("#document-viewer");document.querySelectorAll("#docs-list button").forEach((button,i)=>button.classList.toggle("active",i===index));viewer.innerHTML=`<p>${t("loading_document")}</p>`;try{const response=await fetch(documentInfo.path[language],{cache:"no-store"});if(!response.ok)throw new Error(response.status);const markdown=await response.text();viewer.innerHTML=DOMPurify.sanitize(marked.parse(markdown));viewer.querySelectorAll("a").forEach(link=>{if(link.host&&link.host!==location.host)link.target="_blank"})}catch(_){viewer.innerHTML=`<p>${t("document_error")}</p>`}}
function renderDocumentList(){const language=currentLanguage==="pt-BR"?"pt":"en";const list=document.querySelector("#docs-list");list.innerHTML="";documents.forEach((item,index)=>{const button=document.createElement("button");button.type="button";button.textContent=item.title[language];button.addEventListener("click",()=>showDocument(index));list.appendChild(button)});showDocument(activeDocument)}
document.addEventListener("DOMContentLoaded",renderDocumentList);document.addEventListener("tbx-language",renderDocumentList);
