require_relative './interesting_backtrace_helper'

class InstanceMethod
  def work()
    top_level_hello
  end
end

module ModuleMethod
  def call_instance()
      InstanceMethod.new.work()
  end
end

class IncludedMethod
  include ModuleMethod
end

# Top level method test too (block in main)
def work_main()
  IncludedMethod.new.call_instance()
end

loop do
    work_main()
end
