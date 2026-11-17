// SPDX-License-Identifier: GPL-3.0-or-later
#include <windows.h>
#include <shlobj.h>
#include <wrl/client.h>
#include <vector>

// The Shell supplies the same IDataObject (including CF_HDROP) as Explorer.
// Absolute PIDLs also allow a selection spanning different folders.
static HRESULT sample_data(const wchar_t* const* files, unsigned count, IDataObject** data) {
    std::vector<PCIDLIST_ABSOLUTE> ids;
    for (unsigned i = 0; i < count; ++i) {
        PIDLIST_ABSOLUTE id = nullptr;
        if (FAILED(SHParseDisplayName(files[i], nullptr, &id, 0, nullptr))) break;
        ids.push_back(id);
    }
    HRESULT hr = E_FAIL;
    Microsoft::WRL::ComPtr<IShellItemArray> items;
    if (ids.size() == count && SUCCEEDED(SHCreateShellItemArrayFromIDLists(count, ids.data(), &items))) {
        hr = items->BindToHandler(nullptr, BHID_DataObject, IID_PPV_ARGS(data));
    }
    for (auto id : ids) CoTaskMemFree(const_cast<PIDLIST_ABSOLUTE>(id));
    return hr;
}

extern "C" int ml_drag_sample_files(HWND owner, const wchar_t* const* files, unsigned count) {
    if (!count || !(GetAsyncKeyState(VK_LBUTTON) & 0x8000)) return 0;
    if (FAILED(OleInitialize(nullptr))) return 0;
    DWORD effect = DROPEFFECT_NONE;
    HRESULT hr;
    {
        Microsoft::WRL::ComPtr<IDataObject> data;
        hr = sample_data(files, count, &data);
        if (SUCCEEDED(hr)) hr = SHDoDragDrop(owner, data.Get(), nullptr, DROPEFFECT_COPY, &effect);
    }
    OleUninitialize();
    if (FAILED(hr)) return 0;
    return effect == DROPEFFECT_COPY ? 2 : 1;
}
