// Pinboard installer presentation. Installation and removal remain owned by NSIS.
// All actionable elements remain native Windows controls for keyboard/UI Automation.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <commctrl.h>
#include <dwmapi.h>
#include <uxtheme.h>
#include <algorithm>
#include <string>
#include <vector>
#include "pluginapi.h"

namespace {
HMODULE module;
HWND root, page, progress, details, statusLabel, progressTitle, progressIntro;
HFONT bodyFont, headingFont, smallFont, strongFont, brandFont;
HICON brandIcon;
HBRUSH backgroundBrush, inputBrush, railBrush;
UINT dpi = 96;
int stage, scrollY, extent, pageWidth, pageHeight;
bool korean, removal, busy, highContrast, layoutActive, showDetails, marquee, failed;
std::wstring version;
COLORREF bg, rail, field, text, muted, accent, border;
struct Control { HWND hwnd; int x,y,w,h; std::wstring role; };
std::vector<Control> controls;
std::vector<HWND> chrome;
int px(int v) { return MulDiv(v, static_cast<int>(dpi), 96); }
int logical(int v) { return MulDiv(v, 96, static_cast<int>(dpi)); }
std::wstring pop(stack_t** stack) {
  if (!*stack) return {};
  stack_t* item = *stack; std::wstring value(item->text);
  *stack = item->next; GlobalFree(item); return value;
}
void push(stack_t** stack, int size, const std::wstring& value) {
  auto item = static_cast<stack_t*>(GlobalAlloc(GPTR, sizeof(stack_t) + size * sizeof(wchar_t)));
  if (!item) return;
  lstrcpynW(item->text, value.c_str(), size); item->next = *stack; *stack = item;
}
std::wstring caption(HWND hwnd) {
  int n = GetWindowTextLengthW(hwnd); std::wstring value(n + 1, L'\0');
  GetWindowTextW(hwnd, value.data(), n + 1); value.resize(n); return value;
}
void fill(HDC dc, RECT rect, COLORREF color) {
  HBRUSH brush = CreateSolidBrush(color); FillRect(dc, &rect, brush); DeleteObject(brush);
}
void line(HDC dc, int x1, int y1, int x2, int y2, COLORREF color, int width=1) {
  HPEN pen = CreatePen(PS_SOLID, px(width), color); auto old=SelectObject(dc,pen);
  MoveToEx(dc,x1,y1,nullptr); LineTo(dc,x2,y2); SelectObject(dc,old); DeleteObject(pen);
}
void rounded(HDC dc, RECT rect, COLORREF color, COLORREF outline, int radius=4) {
  HPEN pen=CreatePen(PS_SOLID,1,outline); HBRUSH brush=CreateSolidBrush(color);
  auto oldp=SelectObject(dc,pen); auto oldb=SelectObject(dc,brush);
  RoundRect(dc,rect.left,rect.top,rect.right,rect.bottom,px(radius*2),px(radius*2));
  SelectObject(dc,oldp); SelectObject(dc,oldb); DeleteObject(pen); DeleteObject(brush);
}
HFONT font(int points, int weight=FW_NORMAL) {
  return CreateFontW(-MulDiv(points,dpi,72),0,0,0,weight,FALSE,FALSE,FALSE,
      DEFAULT_CHARSET,OUT_DEFAULT_PRECIS,CLIP_DEFAULT_PRECIS,CLEARTYPE_QUALITY,
      DEFAULT_PITCH,korean ? L"Malgun Gothic" : L"Segoe UI");
}
void makeFonts() {
  for (HFONT h : {bodyFont,headingFont,smallFont,strongFont,brandFont}) if(h) DeleteObject(h);
  bodyFont=font(11); headingFont=font(24,FW_BOLD); smallFont=font(10);
  strongFont=font(11,FW_SEMIBOLD); brandFont=font(18,FW_BOLD);
}
void palette() {
  HIGHCONTRASTW hc{sizeof(hc)}; SystemParametersInfoW(SPI_GETHIGHCONTRAST,sizeof(hc),&hc,0);
  highContrast=(hc.dwFlags&HCF_HIGHCONTRASTON)!=0;
  bg=highContrast?GetSysColor(COLOR_WINDOW):RGB(21,25,30);
  rail=highContrast?bg:RGB(16,20,25); field=highContrast?bg:RGB(32,39,46);
  text=highContrast?GetSysColor(COLOR_WINDOWTEXT):RGB(244,247,250);
  muted=highContrast?text:RGB(165,177,188);
  accent=highContrast?GetSysColor(COLOR_HIGHLIGHT):RGB(114,181,224);
  border=highContrast?text:RGB(67,80,92);
  if(backgroundBrush) DeleteObject(backgroundBrush); if(inputBrush) DeleteObject(inputBrush);
  if(railBrush)DeleteObject(railBrush);
  backgroundBrush=CreateSolidBrush(bg); inputBrush=CreateSolidBrush(field);railBrush=CreateSolidBrush(rail);
}
void setFont(HWND hwnd, HFONT selected) { SendMessageW(hwnd,WM_SETFONT,reinterpret_cast<WPARAM>(selected),TRUE); }
void move(HWND hwnd,int x,int y,int w,int h) {
  if (!IsWindow(hwnd)) return;
  RECT actual; GetWindowRect(hwnd,&actual); MapWindowPoints(nullptr,GetParent(hwnd),reinterpret_cast<POINT*>(&actual),2);
  if(actual.left==px(x) && actual.top==px(y) && actual.right-actual.left==px(w) && actual.bottom-actual.top==px(h)) return;
  SetWindowPos(hwnd,nullptr,px(x),px(y),px(w),px(h),SWP_NOZORDER|SWP_NOACTIVATE);
}
const Control* info(HWND hwnd) {
  for (const auto& c: controls) if(c.hwnd==hwnd) return &c;
  return nullptr;
}
void layout();
LRESULT CALLBACK controlProc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR,DWORD_PTR) {
  wchar_t cls[40]{}; GetClassNameW(hwnd,cls,40);
  const bool button=lstrcmpiW(cls,L"Button")==0;
  const bool edit=lstrcmpiW(cls,L"Edit")==0;
  const bool progressControl=lstrcmpiW(cls,PROGRESS_CLASSW)==0;
  if(edit && !highContrast && msg==WM_NCCALCSIZE) {
    DefSubclassProc(hwnd,msg,wp,lp);
    RECT* r=wp?&reinterpret_cast<NCCALCSIZE_PARAMS*>(lp)->rgrc[0]:reinterpret_cast<RECT*>(lp);
    r->left+=px(10);r->right-=px(10);r->top+=px(9);r->bottom-=px(9);return 0;
  }
  if(edit && !highContrast && msg==WM_NCPAINT) {
    HDC dc=GetWindowDC(hwnd);RECT r;GetWindowRect(hwnd,&r);r.right-=r.left;r.bottom-=r.top;r.left=r.top=0;
    ExcludeClipRect(dc,px(10),px(9),r.right-px(10),r.bottom-px(9));
    rounded(dc,r,field,GetFocus()==hwnd?accent:border);ReleaseDC(hwnd,dc);return 0;
  }
  if(progressControl && !highContrast && msg==WM_PAINT) {
    PAINTSTRUCT ps;HDC dc=BeginPaint(hwnd,&ps);RECT r;GetClientRect(hwnd,&r);fill(dc,r,field);
    RECT bar=r;
    if(marquee){int width=static_cast<int>(r.right);int segment=std::max(px(32),width/4);int offset=static_cast<int>((GetTickCount64()%1800)*(width+segment)/1800)-segment;bar.left=std::max(0,offset);bar.right=std::min(width,offset+segment);}
    else {PBRANGE range{};SendMessageW(hwnd,PBM_GETRANGE,FALSE,reinterpret_cast<LPARAM>(&range));int position=static_cast<int>(SendMessageW(hwnd,PBM_GETPOS,0,0));bar.right=MulDiv(r.right,std::clamp(position-range.iLow,0,std::max(1,range.iHigh-range.iLow)),std::max(1,range.iHigh-range.iLow));}
    if(bar.right>bar.left)fill(dc,bar,failed?RGB(239,155,155):accent);EndPaint(hwnd,&ps);return 0;
  }
  if(msg==WM_SETFOCUS && page && GetParent(hwnd)==page) {
    if(auto c=info(hwnd)) {
      if(c->y < scrollY) scrollY=c->y;
      else if(c->y+c->h > scrollY+pageHeight) scrollY=c->y+c->h-pageHeight+8;
      layout();
    }
  }
  if(button && !highContrast && msg==WM_ERASEBKGND) return 1;
  if(button && !highContrast && msg==WM_PAINT) {
    PAINTSTRUCT ps; HDC dc=BeginPaint(hwnd,&ps); RECT r; GetClientRect(hwnd,&r);
    fill(dc,r,bg); SetBkMode(dc,TRANSPARENT);
    const auto style=GetWindowLongPtrW(hwnd,GWL_STYLE)&BS_TYPEMASK;
    const auto state=SendMessageW(hwnd,BM_GETSTATE,0,0);
    bool enabled=IsWindowEnabled(hwnd), pressed=(state&BST_PUSHED)!=0;
    POINT cursor; GetCursorPos(&cursor); RECT screen; GetWindowRect(hwnd,&screen);
    bool hover=PtInRect(&screen,cursor)!=0;
    bool primary=GetDlgCtrlID(hwnd)==IDOK || (failed && GetDlgCtrlID(hwnd)==IDCANCEL);
    const auto c=info(hwnd); if(c && c->role==L"primary") primary=true;
    SelectObject(dc,strongFont); SetTextColor(dc,enabled?text:muted);
    if(style==BS_AUTOCHECKBOX || style==BS_CHECKBOX || style==BS_AUTORADIOBUTTON || style==BS_RADIOBUTTON) {
      bool radio=style==BS_AUTORADIOBUTTON || style==BS_RADIOBUTTON;
      bool checked=SendMessageW(hwnd,BM_GETCHECK,0,0)==BST_CHECKED;
      RECT box{0,(r.bottom-px(20))/2,px(20),(r.bottom+px(20))/2};
      rounded(dc,box,checked?accent:field,hover?accent:border,radio?10:3);
      if(checked) {
        if(radio) { RECT inner=box; InflateRect(&inner,-px(6),-px(6)); rounded(dc,inner,rail,rail,6); }
        else { line(dc,px(5),box.top+px(10),px(9),box.top+px(14),rail,2); line(dc,px(9),box.top+px(14),px(16),box.top+px(6),rail,2); }
      }
      RECT label=r; label.left=px(32); SelectObject(dc,bodyFont);
      DrawTextW(dc,caption(hwnd).c_str(),-1,&label,DT_LEFT|DT_VCENTER|DT_SINGLELINE);
    } else {
      COLORREF color=primary ? (pressed?RGB(82,151,197):hover?RGB(143,200,235):accent) : (hover?RGB(46,56,66):field);
      if(!enabled) color=field;
      RECT edge=r; --edge.right; --edge.bottom;
      rounded(dc,edge,color,primary&&enabled?color:border);
      SetTextColor(dc,primary&&enabled?rail:enabled?text:muted);
      DrawTextW(dc,caption(hwnd).c_str(),-1,&r,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
    }
    if(GetFocus()==hwnd) { RECT focus=r; InflateRect(&focus,-px(3),-px(3)); SetTextColor(dc,primary?rail:accent); DrawFocusRect(dc,&focus); }
    EndPaint(hwnd,&ps); return 0;
  }
  if(msg==WM_MOUSEMOVE) { TRACKMOUSEEVENT track{sizeof(track),TME_LEAVE,hwnd,0}; TrackMouseEvent(&track); }
  LRESULT result=DefSubclassProc(hwnd,msg,wp,lp);
  if(progressControl && (msg==PBM_SETPOS||msg==PBM_DELTAPOS||msg==PBM_STEPIT||msg==PBM_SETRANGE||msg==PBM_SETRANGE32))InvalidateRect(hwnd,nullptr,FALSE);
  if(edit && (msg==WM_SETFOCUS||msg==WM_KILLFOCUS))RedrawWindow(hwnd,nullptr,nullptr,RDW_FRAME|RDW_INVALIDATE);
  if(button && (msg==WM_MOUSEMOVE||msg==WM_MOUSELEAVE||msg==WM_LBUTTONDOWN||msg==WM_LBUTTONUP||msg==WM_KEYDOWN||msg==WM_KEYUP||msg==WM_SETFOCUS||msg==WM_KILLFOCUS||msg==BM_SETCHECK||msg==WM_ENABLE||msg==WM_SETTEXT)) InvalidateRect(hwnd,nullptr,FALSE);
  if(msg==WM_NCDESTROY) RemoveWindowSubclass(hwnd,controlProc,1);
  return result;
}
void style(HWND hwnd, const std::wstring& role) {
  setFont(hwnd,role==L"heading"?headingFont:role==L"label"?strongFont:role==L"small"?smallFont:bodyFont);
  if(!highContrast) SetWindowTheme(hwnd,L"",L"");
  SetWindowSubclass(hwnd,controlProc,1,0);
  if(role==L"edit") {
    auto ex=GetWindowLongPtrW(hwnd,GWL_EXSTYLE);
    SetWindowLongPtrW(hwnd,GWL_EXSTYLE,highContrast?ex|WS_EX_CLIENTEDGE:ex&~WS_EX_CLIENTEDGE);
    SetWindowPos(hwnd,nullptr,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
  }
  if(role==L"progress" && !highContrast) {
    SetWindowLongPtrW(hwnd,GWL_STYLE,GetWindowLongPtrW(hwnd,GWL_STYLE)&~WS_BORDER);
    SetWindowLongPtrW(hwnd,GWL_EXSTYLE,GetWindowLongPtrW(hwnd,GWL_EXSTYLE)&~(WS_EX_CLIENTEDGE|WS_EX_STATICEDGE));
    SetWindowPos(hwnd,nullptr,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);
  }
}
void scroll(int amount) {
  scrollY=std::clamp(scrollY+amount,0,std::max(0,extent-pageHeight)); layout();
  InvalidateRect(page,nullptr,TRUE);
}
LRESULT colors(HWND hwnd,HDC dc) {
  wchar_t cls[40]{}; GetClassNameW(hwnd,cls,40);
  bool edit=lstrcmpiW(cls,L"Edit")==0;
  auto c=info(hwnd);
  int step=GetDlgCtrlID(hwnd)-5100;
  if(GetParent(hwnd)==root && step>=0 && step<3) {
    SetTextColor(dc,step==stage?accent:step<stage?text:muted);SetBkColor(dc,rail);SetBkMode(dc,OPAQUE);
    return reinterpret_cast<LRESULT>(railBrush);
  }
  SetTextColor(dc,c&&(c->role==L"muted"||c->role==L"small")?muted:c&&c->role==L"error"?RGB(239,155,155):text);
  SetBkColor(dc,edit?field:bg); SetBkMode(dc,OPAQUE);
  return reinterpret_cast<LRESULT>(edit?inputBrush:backgroundBrush);
}
LRESULT CALLBACK pageProc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR,DWORD_PTR) {
  if(msg==WM_CTLCOLORSTATIC||msg==WM_CTLCOLOREDIT||msg==WM_CTLCOLORBTN) return colors(reinterpret_cast<HWND>(lp),reinterpret_cast<HDC>(wp));
  if(msg==WM_ERASEBKGND) { RECT r; GetClientRect(hwnd,&r); fill(reinterpret_cast<HDC>(wp),r,bg); return 1; }
  if(msg==WM_MOUSEWHEEL) { scroll(-GET_WHEEL_DELTA_WPARAM(wp)/WHEEL_DELTA*36); return 0; }
  if(msg==WM_VSCROLL) {
    SCROLLINFO si{sizeof(si),SIF_ALL}; GetScrollInfo(hwnd,SB_VERT,&si);
    switch(LOWORD(wp)) {
      case SB_LINEUP:scroll(-24);break; case SB_LINEDOWN:scroll(24);break;
      case SB_PAGEUP:scroll(-pageHeight);break; case SB_PAGEDOWN:scroll(pageHeight);break;
      case SB_THUMBTRACK:scroll(si.nTrackPos-scrollY);break;
    } return 0;
  }
  if(msg==WM_COMMAND && LOWORD(wp)==1027 && progress) {
    showDetails=!showDetails; ShowWindow(details,showDetails?SW_SHOW:SW_HIDE);
    SetWindowTextW(GetDlgItem(hwnd,1027),korean?(showDetails?L"세부 정보 닫기":L"세부 정보 보기"):(showDetails?L"Hide details":L"Show details"));
    layout(); return 0;
  }
  if(msg==WM_NCDESTROY) { RemoveWindowSubclass(hwnd,pageProc,1); if(hwnd==page){page=nullptr;controls.clear();progress=nullptr;} }
  return DefSubclassProc(hwnd,msg,wp,lp);
}
void drawRoot(HWND hwnd,HDC dc) {
  RECT r; GetClientRect(hwnd,&r); fill(dc,r,bg);
  RECT side=r; side.right=px(220); fill(dc,side,rail);
  line(dc,px(220),r.bottom-px(76),r.right,r.bottom-px(76),border);
  if(brandIcon) DrawIconEx(dc,px(24),px(32),brandIcon,px(56),px(56),0,nullptr,DI_NORMAL);
  SetBkMode(dc,TRANSPARENT); SetTextColor(dc,text); SelectObject(dc,brandFont);
  RECT brand{px(88),px(35),px(216),px(82)}; DrawTextW(dc,L"Pinboard",-1,&brand,DT_LEFT|DT_VCENTER|DT_SINGLELINE);
  for(int i=0;i<3;i++) {
    int top=px(150+i*66); bool active=stage==i, done=stage>i;
    if(i<2) line(dc,px(43),top+px(34),px(43),top+px(60),border);
    RECT circle{px(28),top,px(58),top+px(30)};
    rounded(dc,circle,active||done?accent:rail,active||done?accent:border,15);
    SelectObject(dc,strongFont); SetTextColor(dc,active||done?rail:muted);
    std::wstring number=done?L"✓":std::to_wstring(i+1); DrawTextW(dc,number.c_str(),-1,&circle,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
    RECT label{px(76),top,px(212),top+px(30)};
    SetTextColor(dc,active?accent:done?text:muted);
    const wchar_t* ko[]={removal?L"확인":L"준비",removal?L"제거":L"설치",L"완료"};
    const wchar_t* en[]={removal?L"Review":L"Prepare",removal?L"Uninstall":L"Install",L"Finish"};
    if(chrome.empty()) DrawTextW(dc,korean?ko[i]:en[i],-1,&label,DT_LEFT|DT_VCENTER|DT_SINGLELINE);
  }
  if(r.bottom>px(500) && !highContrast) {
    int y=r.bottom-px(158);
    for(int i=0;i<3;i++) { RECT card{px(28+i*17),y+px(i*14),px(148+i*17),y+px(66+i*14)}; rounded(dc,card,rail,RGB(36,47,58)); for(int j=0;j<3;j++)line(dc,card.left+px(12),card.top+px(20+j*12),card.right-px(12),card.top+px(20+j*12),RGB(31,41,51)); }
  }
  SelectObject(dc,smallFont); SetTextColor(dc,muted);
  RECT ver{px(28),r.bottom-px(35),px(212),r.bottom-px(12)};
  DrawTextW(dc,(L"Pinboard "+version).c_str(),-1,&ver,DT_LEFT|DT_SINGLELINE);
  RECT scope{px(256),r.bottom-px(48),r.right-px(320),r.bottom-px(20)};
  DrawTextW(dc,korean?L"현재 Windows 사용자용":L"For your Windows account",-1,&scope,DT_LEFT|DT_VCENTER|DT_SINGLELINE);
}
void layout() {
  if(!root || layoutActive) return; layoutActive=true;
  RECT r; GetClientRect(root,&r); int width=logical(r.right),height=logical(r.bottom);
  for(int id : {3,1028,1034,1035,1036,1037,1038,1039,1045,1256}) ShowWindow(GetDlgItem(root,id),SW_HIDE);
  move(GetDlgItem(root,IDOK),width-172,height-58,136,40);
  move(GetDlgItem(root,IDCANCEL),width-288,height-58,104,40);
  if(failed) {
    ShowWindow(GetDlgItem(root,IDOK),SW_HIDE);
    auto close=GetDlgItem(root,IDCANCEL);ShowWindow(close,SW_SHOW);EnableWindow(close,TRUE);
    SetWindowTextW(close,korean?L"닫기":L"Close");move(close,width-172,height-58,136,40);
  }
  for(int i=0;i<static_cast<int>(chrome.size());++i) move(chrome[i],76,150+i*66,136,30);
  if(page && IsWindow(page)) {
    pageWidth=width-292; pageHeight=height-120;
    extent=0; for(auto& c:controls) if(IsWindow(c.hwnd) && (GetWindowLongPtrW(c.hwnd,GWL_STYLE)&WS_VISIBLE)) extent=std::max(extent,c.y+c.h+8);
    bool overflow=extent>pageHeight; if(overflow) pageWidth-=logical(GetSystemMetrics(SM_CXVSCROLL));
    scrollY=std::clamp(scrollY,0,std::max(0,extent-pageHeight));
    move(page,256,32,width-292,pageHeight);
    SCROLLINFO si{sizeof(si),SIF_RANGE|SIF_PAGE|SIF_POS,0,std::max(0,extent-1),static_cast<UINT>(pageHeight),scrollY,0};
    SetScrollInfo(page,SB_VERT,&si,TRUE); ShowScrollBar(page,SB_VERT,overflow);
    for(auto& c:controls) {
      int x=c.x<0?pageWidth+c.x:c.x;
      int w=c.w<0?pageWidth-x+c.w:c.w;
      move(c.hwnd,x,c.y-scrollY,std::max(1,w),c.h);
    }
  }
  layoutActive=false;
}
LRESULT CALLBACK rootProc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR,DWORD_PTR) {
  if(msg==WM_PAINT) { PAINTSTRUCT ps; auto dc=BeginPaint(hwnd,&ps); drawRoot(hwnd,dc); EndPaint(hwnd,&ps); return 0; }
  if(msg==WM_ERASEBKGND) return 1;
  if(msg==WM_CTLCOLORSTATIC||msg==WM_CTLCOLOREDIT||msg==WM_CTLCOLORBTN) return colors(reinterpret_cast<HWND>(lp),reinterpret_cast<HDC>(wp));
  if(msg==WM_CLOSE && busy) return 0;
  if(msg==WM_DPICHANGED) {
    dpi=HIWORD(wp); makeFonts(); RECT* r=reinterpret_cast<RECT*>(lp);
    for(auto& c:controls)style(c.hwnd,c.role);
    style(GetDlgItem(root,IDOK),L"primary");style(GetDlgItem(root,IDCANCEL),L"button");
    for(auto hwnd:chrome)setFont(hwnd,strongFont);
    MONITORINFO monitor{sizeof(monitor)}; GetMonitorInfoW(MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST),&monitor);
    int w=std::min(r->right-r->left,monitor.rcWork.right-monitor.rcWork.left-16);
    int h=std::min(r->bottom-r->top,monitor.rcWork.bottom-monitor.rcWork.top-16);
    SetWindowPos(hwnd,nullptr,std::clamp(r->left,monitor.rcWork.left,monitor.rcWork.right-w),std::clamp(r->top,monitor.rcWork.top,monitor.rcWork.bottom-h),w,h,SWP_NOZORDER|SWP_NOACTIVATE);
    layout(); InvalidateRect(hwnd,nullptr,TRUE); return 0;
  }
  if(msg==WM_SIZE) layout();
  if(msg==WM_SETTINGCHANGE) { palette();for(auto& c:controls)style(c.hwnd,c.role);layout();InvalidateRect(hwnd,nullptr,TRUE); }
  if(msg==WM_TIMER && wp==0x5042) { layout();if(marquee&&progress)InvalidateRect(progress,nullptr,FALSE);return 0; }
  if(msg==WM_NCDESTROY) { KillTimer(hwnd,0x5042); RemoveWindowSubclass(hwnd,rootProc,1); root=nullptr; }
  return DefSubclassProc(hwnd,msg,wp,lp);
}
UINT_PTR callback(NSPIM message) {
  if(message==NSPIM_GUIUNLOAD || message==NSPIM_UNLOAD) {
    if(root) { KillTimer(root,0x5042); RemoveWindowSubclass(root,rootProc,1); }
    root=nullptr; page=nullptr; controls.clear();chrome.clear();
    for(HFONT h:{bodyFont,headingFont,smallFont,strongFont,brandFont}) if(h) DeleteObject(h);
    bodyFont=headingFont=smallFont=strongFont=brandFont=nullptr;
    if(brandIcon)DestroyIcon(brandIcon);brandIcon=nullptr;
    if(backgroundBrush)DeleteObject(backgroundBrush);backgroundBrush=nullptr;
    if(inputBrush)DeleteObject(inputBrush);inputBrush=nullptr;
    if(railBrush)DeleteObject(railBrush);railBrush=nullptr;
  } return 0;
}
void stageState(int value) {
  stage=value; busy=value==1;
  EnableWindow(GetDlgItem(root,IDCANCEL),!busy);
  EnableMenuItem(GetSystemMenu(root,FALSE),SC_CLOSE,MF_BYCOMMAND|(busy?MF_GRAYED:MF_ENABLED));
  InvalidateRect(root,nullptr,TRUE);
  for(auto hwnd:chrome)InvalidateRect(hwnd,nullptr,TRUE);
}
void begin(HWND hwnd,int value) {
  page=hwnd;controls.clear();scrollY=0;extent=0;progress=nullptr;showDetails=false;marquee=false;failed=false;
  SetWindowLongPtrW(page,GWL_STYLE,GetWindowLongPtrW(page,GWL_STYLE)|WS_CLIPCHILDREN);
  stageState(value); SetWindowSubclass(page,pageProc,1,0); layout();
}
void place(HWND hwnd,int x,int y,int w,int h,const std::wstring& role) {
  controls.push_back({hwnd,x,y,w,h,role});style(hwnd,role);layout();
}
}

#define EXPORT(name) extern "C" __declspec(dllexport) void __cdecl name(HWND parent,int stringSize,wchar_t*,stack_t** stack,extra_parameters* extra)
EXPORT(Init) {
  removal=pop(stack)==L"remove";version=pop(stack); auto icon=pop(stack); korean=pop(stack)==L"1042";
  root=parent; dpi=GetDpiForWindow(root); if(!dpi)dpi=96;
  palette();makeFonts();brandIcon=static_cast<HICON>(LoadImageW(nullptr,icon.c_str(),IMAGE_ICON,px(64),px(64),LR_LOADFROMFILE));
  extra->RegisterPluginCallback(module,callback);
  SetWindowSubclass(root,rootProc,1,0);
  SetWindowLongPtrW(root,GWL_STYLE,GetWindowLongPtrW(root,GWL_STYLE)|WS_CLIPCHILDREN);
  const wchar_t* ko[]={removal?L"확인":L"준비",removal?L"제거":L"설치",L"완료"};
  const wchar_t* en[]={removal?L"Review":L"Prepare",removal?L"Uninstall":L"Install",L"Finish"};
  for(int i=0;i<3;i++){auto label=CreateWindowExW(0,L"Static",korean?ko[i]:en[i],WS_CHILD|WS_VISIBLE|SS_CENTERIMAGE,0,0,0,0,root,reinterpret_cast<HMENU>(static_cast<INT_PTR>(5100+i)),module,nullptr);chrome.push_back(label);setFont(label,strongFont);}
  BOOL dark=!highContrast;DwmSetWindowAttribute(root,20,&dark,sizeof(dark));
  RECT rect{0,0,px(900),px(560)};
  AdjustWindowRectExForDpi(&rect,static_cast<DWORD>(GetWindowLongPtrW(root,GWL_STYLE)),FALSE,static_cast<DWORD>(GetWindowLongPtrW(root,GWL_EXSTYLE)),dpi);
  MONITORINFO monitor{sizeof(monitor)};GetMonitorInfoW(MonitorFromWindow(root,MONITOR_DEFAULTTONEAREST),&monitor);
  int w=std::min(rect.right-rect.left,monitor.rcWork.right-monitor.rcWork.left-16);
  int h=std::min(rect.bottom-rect.top,monitor.rcWork.bottom-monitor.rcWork.top-16);
  SetWindowPos(root,nullptr,monitor.rcWork.left+(monitor.rcWork.right-monitor.rcWork.left-w)/2,monitor.rcWork.top+(monitor.rcWork.bottom-monitor.rcWork.top-h)/2,w,h,SWP_NOZORDER|SWP_NOACTIVATE);
  style(GetDlgItem(root,IDOK),L"primary");style(GetDlgItem(root,IDCANCEL),L"button");
  SetTimer(root,0x5042,120,nullptr);layout();
}
EXPORT(Begin) { auto hwnd=reinterpret_cast<HWND>(static_cast<INT_PTR>(std::stoll(pop(stack))));int value=std::stoi(pop(stack));begin(hwnd,value); }
EXPORT(Place) { auto hwnd=reinterpret_cast<HWND>(static_cast<INT_PTR>(std::stoll(pop(stack))));int x=std::stoi(pop(stack)),y=std::stoi(pop(stack)),w=std::stoi(pop(stack)),h=std::stoi(pop(stack));auto role=pop(stack);place(hwnd,x,y,w,h,role); }
EXPORT(State) { stageState(std::stoi(pop(stack))); }
EXPORT(Reveal) {
  auto hwnd=reinterpret_cast<HWND>(static_cast<INT_PTR>(std::stoll(pop(stack))));
  if(auto c=info(hwnd)){int bottom=c->y+c->h+8;layout();scrollY=std::max(0,bottom-pageHeight);layout();InvalidateRect(page,nullptr,TRUE);}
}
EXPORT(Mode) {
  removal=pop(stack)==L"remove";
  const wchar_t* ko[]={removal?L"확인":L"준비",removal?L"제거":L"설치",L"완료"};
  const wchar_t* en[]={removal?L"Review":L"Prepare",removal?L"Uninstall":L"Install",L"Finish"};
  for(int i=0;i<static_cast<int>(chrome.size());++i)SetWindowTextW(chrome[i],korean?ko[i]:en[i]);
  InvalidateRect(root,nullptr,TRUE);
}
EXPORT(Progress) {
  auto title=pop(stack),subtitle=pop(stack);HWND inner=FindWindowExW(root,nullptr,L"#32770",nullptr);
  if(!inner)return;begin(inner,1);progress=GetDlgItem(inner,1004);details=GetDlgItem(inner,1016);statusLabel=GetDlgItem(inner,1006);
  progressTitle=CreateWindowExW(0,L"Static",title.c_str(),WS_CHILD|WS_VISIBLE,0,0,0,0,inner,nullptr,module,nullptr);
  progressIntro=CreateWindowExW(0,L"Static",subtitle.c_str(),WS_CHILD|WS_VISIBLE,0,0,0,0,inner,nullptr,module,nullptr);
  place(progressTitle,0,0,-1,52,L"heading");place(progressIntro,0,58,-1,44,L"muted");
  place(statusLabel,0,142,-1,34,L"text");place(progress,0,191,-1,8,L"progress");
  SendMessageW(progress,PBM_SETBARCOLOR,0,accent);SendMessageW(progress,PBM_SETBKCOLOR,0,field);
  HWND toggle=GetDlgItem(inner,1027);place(toggle,0,228,152,36,L"button");
  SetWindowTextW(toggle,korean?L"세부 정보 보기":L"Show details");
  place(details,0,280,-1,140,L"small");
  ListView_SetBkColor(details,field);ListView_SetTextBkColor(details,field);ListView_SetTextColor(details,text);
  ShowWindow(details,SW_HIDE);layout();
}
EXPORT(Marquee) { marquee=true;if(progress){SetWindowLongPtrW(progress,GWL_STYLE,GetWindowLongPtrW(progress,GWL_STYLE)|PBS_MARQUEE);SendMessageW(progress,PBM_SETMARQUEE,TRUE,35);} }
EXPORT(Error) { stageState(0);if(progress){SendMessageW(progress,PBM_SETMARQUEE,FALSE,0);SendMessageW(progress,PBM_SETBARCOLOR,0,RGB(239,155,155));} }
EXPORT(Failure) { auto title=pop(stack),intro=pop(stack);busy=false;marquee=false;failed=true;EnableWindow(GetDlgItem(root,IDCANCEL),TRUE);EnableMenuItem(GetSystemMenu(root,FALSE),SC_CLOSE,MF_BYCOMMAND|MF_ENABLED);SetWindowTextW(GetDlgItem(root,IDCANCEL),korean?L"닫기":L"Close");SetWindowTextW(progressTitle,title.c_str());SetWindowTextW(progressIntro,intro.c_str());if(progress){SendMessageW(progress,PBM_SETMARQUEE,FALSE,0);SendMessageW(progress,PBM_SETBARCOLOR,0,RGB(239,155,155));}showDetails=true;ShowWindow(details,SW_SHOW);SetWindowTextW(GetDlgItem(page,1027),korean?L"세부 정보 닫기":L"Hide details");layout();InvalidateRect(root,nullptr,TRUE); }
BOOL WINAPI DllMain(HINSTANCE instance,DWORD reason,LPVOID) { if(reason==DLL_PROCESS_ATTACH){module=instance;DisableThreadLibraryCalls(instance);}return TRUE; }
