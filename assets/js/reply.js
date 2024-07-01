document.addEventListener("DOMContentLoaded", () => {
  /////////// Elements ///////////
  const zipFileInput = document.getElementById("zip-input")
  const folderFileInput = document.getElementById("folder-input")

  const excelList = document.getElementById("excel-list")
  const excelFileInput = document.getElementById("excel-file")
  const submitExcelButton = document.getElementById("submit-excel")
  const templateButton = document.getElementById("download-template")
  const templateFileInput = document.getElementById("template-input")
  const cellReply = document.getElementById("cell-reply")

  let cutRows = []
  let rename = []
  let checked = []

  let loading = false
  let formData = new FormData()

  function updateTotalCount() {
    let total_count = document.getElementById("total-count")
    total_count.textContent = `Total ${excelList.children.length} files`
  }

  /////////// ______ ///////////
  console.log("using version 4.1.0")


  folderFileInput.addEventListener("change", async (e) => {
    const files = e.target.files
    for (file of files) {
      const newFile = new File([file], file.name.replace(/^.*[\\/]/, ''), {type: file.type})
      createFileInExcelList(newFile)
    }
  })

  zipFileInput.addEventListener("change", async (e) => {
    const zipReader = new zip.ZipReader(new zip.BlobReader(e.target.files[0]))
    const entries = await zipReader.getEntries()
    entries.forEach(async (entry) => {
      let writer
      let mime

      if (entry.filename.endsWith(".xlsx")) {
        writer = new zip.BlobWriter(
          "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        mime =
          "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      } else if (entry.filename.endsWith(".xls")) {
        writer = new zip.BlobWriter("application/vnd.ms-excel")
        mime = "application/vnd.ms-excel"
      }

      let blob = new Blob([await entry.getData(writer)], {
        type: mime,
      })
      let file = new File([blob], entry.filename, {
        type: mime,
      })

      createFileInExcelList(file)
    })
  })

  templateFileInput.addEventListener("change", (e) => {
    console.log("processing template")
    let file = e.target.files[0]
    let reader = new FileReader()
    let rowsLength;

    reader.onload = function (e) {
      var data = e.target.result 
      var workbook = XLSX.read(data, {
        type: "binary",
      })

      var template_sheet = XLSX.utils.sheet_to_json(workbook.Sheets[workbook.SheetNames[0]], {header: 1}).filter(subArray => subArray.length > 0); 
      template_sheet.shift()

      // clean 
      let files = formData.getAll("excel-file[]")
      formData.delete("excel-file[]")

      cutRows = []
      checked = []

      console.log(template_sheet)

      let cutRowsInputs = document.querySelectorAll("#cut-row")
      let fileNameInputs = document.querySelectorAll("#filename-input")
      let checkboxes = document.querySelectorAll('input[type="checkbox"]');

      template_sheet.forEach((row, idx) => {
        let ext; 
        
        // get the right ext
        if (files[idx].type == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet") {
          ext = ".xlsx"
        } else if (files[idx].type == "application/vnd.ms-excel") {
          ext = ".xls"
        }

        const newFile = new File([files[idx]], row[1] + ext, { type: files[idx].type })
        formData.append("excel-file[]", newFile)

        cutRows.push(row[5])
        checked.push(Boolean(row[6]))

        // update the dom 
        cutRowsInputs[idx].value =  row[5]
        fileNameInputs[idx].value = row[1]

        // skip the cell reply (find a better approach :P)
        checkboxes[idx+1].checked = Boolean(row[6])
      })
    }

    reader.onerror = function () {
      // Handle FileReader errors
      alert("Error reading the file.")
      throw new Error("FileReader error")
    }

    reader.readAsBinaryString(file)
  })

  excelFileInput.addEventListener("change", async (e) => {
    const files = e.target.files
    for (file of files) {
      createFileInExcelList(file)
    }
  })

  submitExcelButton.addEventListener("click", async (e) => {
    e.preventDefault()

    loading = true

    let mark = document.getElementById("mark")
    mark.style.display = "flex"

    formData.delete("last-mod[]")
    // append last mod
    formData
      .getAll("excel-file[]")
      .map((file) => {
        let lastMod = file.lastModified
        const date = getDate(lastMod, false)

        return date
      })
      .forEach((date) => {
        formData.append("last-mod[]", date)
      })

    // append sizes
    formData
      .getAll("excel-file[]")
      .map((file) => {
        let size = file.size
        return size
      })
      .forEach((size) => {
        formData.append("size[]", size)
      })

    console.log("logging cut rows")
    console.log(cutRows)

    // append cut row
    formData.getAll("excel-file[]").forEach((_, idx) => {
      if (cutRows[idx] != undefined) {
        formData.append("cut-row[]", cutRows[idx])
      } else {
        formData.append("cut-row[]", 0)
      }

      if (rename[idx] != undefined){
        formData.append("rename[]", rename[idx])
      } else {
        formData.append("rename[]", false)
      }

      if (checked[idx] != undefined){
        formData.append("checked[]", checked[idx])
      } else {
        formData.append("checked[]", false)
      }
    })

    console.log(formData)

    console.log(formData.getAll("excel-file[]"))
    console.log("We're sending a request to the server.")

    const res = await fetch(`/api/reply?reply=${cellReply.checked == true ? "true" : "false"}`, {
      method: "POST",
      body: formData,
    })
    if (!res.ok) {
      mark.style.display = "none"
      const error = await res.json()
      alert(error.error)
      throw error
    }

    console.log("Received the response, parsing and downloading the output.")

    var date = new Date()
    var time = new Date()
      .toLocaleTimeString("en-US", {
        timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        hour12: false,
        hour: "numeric",
        minute: "numeric",
      })
      .replace(":", "")

    console.log(time)
    const blob = await res.blob()
    const url = window.URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = `search${formatDate(date, "mmddyy")}${time.trim()}.zip`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    window.URL.revokeObjectURL(url)

    console.log("Done!")

    // cleanup
    loading = false
    mark.style.display = "none"
    excelList.textContent = ""
    formData = new FormData()

    // turn this off when debuggging
    updateTotalCount()
    location.reload()
  })

  templateButton.addEventListener("click", async (e) => {
    e.preventDefault()

    let templateFormData = formData

    templateFormData.delete("last-mod[]")
    templateFormData.delete("size[]")

    // append last mod
    templateFormData
      .getAll("excel-file[]")
      .map((file) => {
        let lastMod = file.lastModified
        const date = getDate(lastMod, false)

        return date
      })
      .forEach((date) => {
        templateFormData.append("last-mod[]", date)
      })

    // append sizes
    templateFormData
      .getAll("excel-file[]")
      .map((file) => {
        let size = file.size
        return size
      })
      .forEach((size) => {
        templateFormData.append("size[]", size)
      })

    templateFormData.delete("cut-row[]")
    templateFormData.delete("checked[]")
    // append cut row
    templateFormData.getAll("excel-file[]").forEach((_, idx) => {
      if (cutRows[idx] != undefined) {
        templateFormData.append("cut-row[]", cutRows[idx])
      } else {
        templateFormData.append("cut-row[]", 0)
      }

      if (checked[idx] != undefined){
        formData.append("checked[]", checked[idx])
        console.log("pushing")
        console.log(checked)
      } else {
        formData.append("checked[]", false)
        console.log("pushing")
        console.log(checked)
      }
    })


    const res = await fetch("/api/reply-template", {
      method: "POST",
      body: templateFormData,
    })
    if (!res.ok) {
      const error = await res.json()
      alert(error.error)
      throw error
    }

    const blob = await res.blob()
    const url = window.URL.createObjectURL(blob)
    const a = document.createElement("a")
    a.href = url
    a.download = "template.xlsx"
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    window.URL.revokeObjectURL(url)
  })

  function createFileInExcelList(file) {
    formData.append("excel-file[]", file)

    const div = document.createElement("div")
    const li = document.createElement("li")
    const a = document.createElement("a")
    const getFileButton = document.createElement("button")
    const cutRow = document.createElement("input")
    const fileNameInput = document.createElement("input");
    const fileExtLabel = document.createElement("label");
    const fileNameDiv = document.createElement("div");
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox"

    getFileButton.textContent = "Get File"
    
    cutRow.id = "cut-row"

    fileNameInput.id = "filename-input"
    fileNameInput.value = file.name.split(".")[0]
    fileNameInput.size = "80"
    fileExtLabel.innerText = file.name.split(".")[1]

    fileNameDiv.appendChild(checkbox)
    fileNameDiv.appendChild(fileNameInput)
    fileNameDiv.appendChild(fileExtLabel)

    checkbox.addEventListener("click", e => {
      let li = e.target.closest("li")
      let nodes = Array.from(li.closest("ul").children)
      let index = nodes.indexOf(li)

      checked[index] = e.target.checked
      console.log(checked)
    })

    fileNameInput.addEventListener("change", (e) => {
      let li = e.target.closest("li")
      let nodes = Array.from(li.closest("ul").children)
      let index = nodes.indexOf(li)

      const newFileName = e.target.value + file.name.slice(file.name.lastIndexOf("."));
      const newFile = new File([file], newFileName, { type: file.type });

      if (newFileName != file.name) {
        rename[index] = true
      } else {
        rename[index] = false
      }

      console.log(rename)

      let values = formData.getAll("excel-file[]")
      values[index] = newFile
      console.log(formData.getAll("excel-file[]"))
      formData.delete("excel-file[]")

      values.forEach((value, _) => {
        formData.append("excel-file[]", value)
      })

      a.download = newFileName;
      a.textContent = newFileName;
    });

    cutRow.addEventListener("change", e => {
      e.preventDefault();
      const val = e.target.value;

      let li = e.target.closest("li");
      let nodes = Array.from(li.closest("ul").children);
      let index = nodes.indexOf(li);

      cutRows[index] = val
      console.log(cutRows)
    });

    getFileButton.addEventListener("click", async (e) => {
      e.preventDefault()
      // loading
      loading = true
      let mark = document.getElementById("mark")
      mark.style.display = "flex"


      // find which one we clicked
      let li = e.target.closest("li");
      let nodes = Array.from(li.closest("ul").children);
      let index = nodes.indexOf(li);

      // make a new formdata to not interfere with the original one
      let clickedFileEntry = formData.getAll("excel-file[]")[index]
      let singleFormData = new FormData()
      singleFormData.append("excel-file[]", clickedFileEntry)

      console.log(clickedFileEntry)
      for (let pair of singleFormData.entries()) {
        console.log(pair[0]+ ', ' + pair[1]); 
      }

      // append last mod
      singleFormData
        .getAll("excel-file[]")
        .map((file) => {
          let lastMod = file.lastModified
          const date = getDate(lastMod, false)

          return date
        })
        .forEach((date) => {
          singleFormData.append("last-mod[]", date)
        })


      // append cut row
      if (cutRows[index] != undefined) {
        singleFormData.append("cut-row[]", cutRows[index])
      } else {
        singleFormData.append("cut-row[]", 0)
      }

      // append rename 
      if (rename[index] != undefined) {
        singleFormData.append("rename[]", rename[index])
      } else {
        singleFormData.append("rename[]", false)
      }

      singleFormData.set("checked[]", true)

      // append size
      singleFormData.set("size[]", clickedFileEntry.size)

      const res = await fetch(`/api/reply-single?reply=${cellReply.checked == true ? "true" : "false"}`, {
        method: "POST",
        body: singleFormData,
      })

      if (!res.ok) {
        const error = await res.json()
        alert(error.error)
        throw error
      }

      const blob = await res.blob()
      const url = window.URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = clickedFileEntry.name
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      window.URL.revokeObjectURL(url)

      loading = false
      mark.style.display = "none"

      console.log("Done!")
    })

    // li.textContent = file.name
    li.appendChild(fileNameDiv)
    excelList.appendChild(li)

    const button = document.createElement("button")
    button.textContent = "X"
    button.addEventListener("click", (e) => {
      e.preventDefault()
      // TODO refactor
      let li = e.target.closest("li")
      let nodes = Array.from(li.closest("ul").children)
      let index = nodes.indexOf(li)

      let values = formData.getAll("excel-file[]")
      values.splice(index, 1)
      formData.delete("excel-file[]")

      values.forEach((value, _) => {
        formData.append("excel-file[]", value)
      })


      cutRows.splice(index, 1)
      formData.delete("cut-row[]")

      cutRows.forEach((value, _) => {
        formData.append("cut-row[]", value)
      })

      console.log(cutRows)

      checked.splice(index, 1)
      formData.delete("checked[]")

      checked.forEach((value, _) => {
        formData.append("checked[]", value)
      })

      console.log(checked)

      let size_values = formData.getAll("size[]")
      size_values.splice(index, 1)
      formData.delete("size[]")

      size_values.forEach((value, _) => {
        formData.append("size[]", value)
      })

      li.remove()
      updateTotalCount()
    })

    div.appendChild(cutRow)
    div.appendChild(button)
    div.appendChild(getFileButton)

    li.appendChild(div)

    updateTotalCount()
  }
})
