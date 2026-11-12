# frozen_string_literal: true

# AgriCUDa kernel integration for APSInsight.
# AgriCUDa (SbruiceS/AgriCUDa) is an agriculture-oriented programming language.
# Real capabilities (from source):
#   - agricuda check / run / ir / doctor
#   - kernel fn + launch <<<grid, block>>> syntax
#   - CPU device execution of kernels (blocks in order)
#   - GPU and FPGA backends are NOT implemented
# Results must come from a real agricuda binary or an explicit test double.
# Never fabricate numeric kernel results.

class AgricudaAdapter
  class Unavailable < StandardError; end
  class ExecutionError < StandardError; end

  KNOWN_KERNELS = %w[saxpy degree_day soil_bucket].freeze

  def available?
    system("which agricuda > /dev/null 2>&1") || ENV["AGRICUDA_BIN"].present?
  end

  def capabilities
    {
      available: available?,
      language: "AgriCUDa",
      version: "0.1",
      features: [
        "check (lex/parse/type)",
        "run (bytecode + stack machine)",
        "ir (disassemble)",
        "kernel fn + launch <<<grid, block>>>",
        "CPU device (blocks in order)",
        "parallel_for host loops"
      ],
      not_implemented: ["GPU backend", "FPGA backend", "native object code", "build"],
      note: available? ? "agricuda binary found" : "agricuda binary not on PATH — test double mode only"
    }
  end

  def run(source)
    raise Unavailable, "agricuda binary not available" unless available?

    bin = ENV.fetch("AGRICUDA_BIN", "agricuda")
    require "open3"
    stdout, stderr, status = Open3.capture3(bin, "run", "-", stdin_data: source)
    raise ExecutionError, stderr unless status.success?

    {
      source_preview: source.lines.first(5).join,
      stdout: stdout,
      value: stdout.strip,
      status: "succeeded"
    }
  end

  def check(source)
    raise Unavailable, "agricuda binary not available" unless available?

    bin = ENV.fetch("AGRICUDA_BIN", "agricuda")
    require "open3"
    stdout, stderr, status = Open3.capture3(bin, "check", "-", stdin_data: source)
    {
      ok: status.success?,
      output: stdout + stderr
    }
  end

  # Explicit test double — never returns invented numeric results
  def run_double(source)
    {
      source_preview: source.lines.first(3).join,
      stdout: "",
      value: nil,
      status: "test_double",
      note: "AgriCUDa binary not present. No kernel result fabricated. Install agricuda and set PATH or AGRICUDA_BIN."
    }
  end
end
