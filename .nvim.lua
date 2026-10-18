vim.lsp.config["rust-analyzer"] = vim.tbl_deep_extend("force", vim.lsp.config["rust-analyzer"] or {}, {
  settings = {
    ["rust-analyzer"] = {
      cargo = {
        target = "wasm32-unknown-unknown",
      },
    },
  },
})
