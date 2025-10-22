# frozen_string_literal: true

# backtracie: Ruby gem for beautiful backtraces
# Copyright (C) 2021 Ivo Anjo <ivo@ivoanjo.me>
#
# This file is part of backtracie.
#
# backtracie is free software: you can redistribute it and/or modify
# it under the terms of the GNU Lesser General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# backtracie is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with backtracie.  If not, see <http://www.gnu.org/licenses/>.

# The following tries to reproduce the most interesting (contrived?) call stack I can think of, with as many weird
# variants as possible.
#
# It is implemented as a separate thread that receives procs to execute and writes back their results. It on purpose
# executes at the top-level, so we can see cases where methods are called on the "main" object.

SAMPLE_REQUESTS_QUEUE = Queue.new
SAMPLE_RESPONSES_QUEUE = Queue.new

def sample_interesting_backtrace(&block)
  SAMPLE_REQUESTS_QUEUE.push(block)
  SAMPLE_RESPONSES_QUEUE.pop
end

# ----

class ClassA
  def hello
    loop do
      sleep 0.5
    end
  end
end

module ModuleB
  class ClassB < ClassA
    def hello
      super
    end
  end
end

module ModuleC
  def self.hello
    ModuleB::ClassB.new.hello
  end
end

class ClassWithStaticMethod
  def self.hello
    ModuleC.hello
  end
end

module ModuleD
  def hello
    ClassWithStaticMethod.hello
  end
end

class ClassC
  include ModuleD
end

$a_proc = proc { ClassC.new.hello }

$a_lambda = lambda { $a_proc.call }

class ClassD; end

$class_d_object = ClassD.new

def $class_d_object.hello
  $a_lambda.call
end

class ClassE
  def hello
    $class_d_object.hello
  end
end

class ClassG
  def hello
    raise "This should not be called"
  end
end

module ContainsRefinement
  module RefinesClassG
    refine ClassG do
      def hello
        if RUBY_VERSION >= "2.7.0"
          ClassE.instance_method(:hello).bind_call(ClassF.new)
        else
          ClassE.instance_method(:hello).bind(ClassF.new).call
        end
      end
    end
  end
end

module ModuleE
  using ContainsRefinement::RefinesClassG

  def self.hello
    ClassG.new.hello
  end
end

class ClassH
  def method_missing(name, *_)
    super unless name == :hello

    ModuleE.hello
  end
end

class ClassF < ClassE
  def hello(arg1, arg2, test1, test2)
    1.times {
      ClassH.new.hello
    }
  end
end

ClassI = Class.new do
  define_method(:hello) do
    ClassF.new.hello(0, 1, 2, 3)
  end
end

$singleton_class = Object.new.singleton_class

def $singleton_class.hello
  ClassI.new.hello
end

$anonymous_instance = Class.new do
  def hello
    $singleton_class.hello
  end
end.new

$anonymous_module = Module.new do
  def self.hello
    $anonymous_instance.hello
  end
end

def method_with_complex_parameters(a, b = nil, *c, (d), f:, g: nil, **h, &i)
  $anonymous_module.hello
end

class ClassJ
  def hello_helper
    yield
  end

  def hello
    hello_helper do
      hello_helper do
        method_with_complex_parameters(0, 1, 2, [3, 4], f: 5, g: 6, h: 7, &proc {})
      end
    end
  end
end

class ClassK
  def hello
    eval("ClassJ.new.hello", binding, __FILE__, __LINE__)
  end
end

class ClassL
  def hello
    ClassK.new.send(:instance_eval, "hello")
  end
end

class ClassM
  def hello
    ClassL.new.send(:eval, "hello")
  end
end

ClassN = Class.new do
  define_method(:hello) do
    1.times {
      ClassM.new.hello
    }
  end
end

def top_level_hello
  ClassN.new.hello
end
